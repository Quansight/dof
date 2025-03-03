use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{PathBuf, Path};
use std::str::FromStr;
use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use reqwest::Client;
use url::Url;

use rattler_conda_types::{Platform, Arch};
use rattler_lock::{CondaBinaryData, CondaPackageData, LockedPackage, PackageHashes, PypiIndexes, PypiPackageData, PypiPackageEnvironmentData, UrlOrPath};
use rattler_digest::{Md5Hash, Sha256Hash};

use uv_cache::Cache;
use uv_client::{RegistryClient, RegistryClientBuilder, FlatIndexClient, Connectivity};
use uv_configuration::{
    ConfigSettings,
    BuildOptions,
    Constraints,
    Concurrency,
    KeyringProviderType,
    SourceStrategy,
    IndexStrategy,
    PreviewMode,
};
use uv_distribution::{RegistryWheelIndex, DistributionDatabase};
use uv_distribution_types::{
    IndexLocations,
    IndexUrl,
    Index,
    CachedDist,
    Dist,
    DependencyMetadata,
    CachedRegistryDist,
    InstalledDist,
    Name,
    Resolution,
};
use uv_dispatch::{SharedState, BuildDispatch};
use uv_install_wheel::LinkMode;
use uv_resolver::FlatIndex;
use uv_installer::{SitePackages, Installer, Preparer, UninstallError};
use uv_normalize::PackageName;
use uv_pep508::VerbatimUrl;
use uv_platform_tags::Tags;
use uv_python::{Interpreter, PythonEnvironment};
use uv_types::{HashStrategy, InFlight, BuildIsolation};

mod rattler_uv_interop;
mod gitutil;

use crate::gitutil::{
    LockedGitUrl,
    to_parsed_git_url,
};

use crate::rattler_uv_interop::{
    convert_to_dist,
    strip_direct_scheme,
    to_uv_version,
    check_url_freshness,
};

/// Provide an iterator over the installed distributions
/// This trait can also be used to mock the installed distributions for testing purposes
pub trait InstalledDistProvider<'a> {
    /// Provide an iterator over the installed distributions
    fn iter(&'a self) -> impl Iterator<Item = &'a InstalledDist>;
}

impl<'a> InstalledDistProvider<'a> for SitePackages {
    fn iter(&'a self) -> impl Iterator<Item = &'a InstalledDist> {
        self.iter()
    }
}

/// Provides a way to get the potentially cached distribution, if it exists
/// This trait can also be used to mock the cache for testing purposes
pub trait CachedDistProvider<'a> {
    /// Get the cached distribution for a package name and version
    fn get_cached_dist(
        &mut self,
        name: &'a uv_normalize::PackageName,
        version: uv_pep440::Version,
    ) -> Option<CachedRegistryDist>;
}

impl<'a> CachedDistProvider<'a> for RegistryWheelIndex<'a> {
    fn get_cached_dist(
        &mut self,
        name: &'a uv_normalize::PackageName,
        version: uv_pep440::Version,
    ) -> Option<CachedRegistryDist> {
        let index = self
            .get(name)
            .find(|entry| entry.dist.filename.version == version);
        index.map(|index| index.dist.clone())
    }
}


#[derive(Debug)]
pub struct InstallPlan {
    /// The distributions that are not already installed in the current
    /// environment, but are available in the local cache.
    pub local: Vec<CachedDist>,

    /// The distributions that are not already installed in the current
    /// environment, and are not available in the local cache.
    /// this is where we differ from UV because we want already have the URL we
    /// want to download
    pub remote: Vec<Dist>,

    /// Any distributions that are already installed in the current environment,
    /// but will be re-installed (including upgraded) to satisfy the
    /// requirements.
    pub reinstalls: Vec<InstalledDist>,

    /// Any distributions that are already installed in the current environment,
    /// and are _not_ necessary to satisfy the requirements.
    pub extraneous: Vec<InstalledDist>,
}

struct InstallPlanner {
    uv_cache: Cache,
    lock_file_dir: PathBuf,
}

enum ValidateCurrentInstall {
    /// Keep this package
    Keep,
    /// Reinstall this package
    Reinstall,
}


/// A wrapper around `Url` which represents a "canonical" version of an original URL.
///
/// A "canonical" url is only intended for internal comparison purposes. It's to help paper over
/// mistakes such as depending on `github.com/foo/bar` vs. `github.com/foo/bar.git`.
///
/// This is **only** for internal purposes and provides no means to actually read the underlying
/// string value of the `Url` it contains. This is intentional, because all fetching should still
/// happen within the context of the original URL.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct CanonicalUrl(Url);

impl CanonicalUrl {
    pub fn new(url: &Url) -> Self {
        let mut url = url.clone();

        // If the URL cannot be a base, then it's not a valid URL anyway.
        if url.cannot_be_a_base() {
            return Self(url);
        }

        // If the URL has no host, then it's not a valid URL anyway.
        if !url.has_host() {
            return Self(url);
        }

        // Strip credentials.
        let _ = url.set_password(None);
        let _ = url.set_username("");

        // Strip a trailing slash.
        if url.path().ends_with('/') {
            url.path_segments_mut()
                .expect("url should be a base")
                .pop_if_empty();
        }

        // For GitHub URLs specifically, just lower-case everything. GitHub
        // treats both the same, but they hash differently, and we're gonna be
        // hashing them. This wants a more general solution, and also we're
        // almost certainly not using the same case conversion rules that GitHub
        // does. (See issue #84)
        if url.host_str() == Some("github.com") {
            url.set_scheme(url.scheme().to_lowercase().as_str())
                .expect("we should be able to set scheme");
            let path = url.path().to_lowercase();
            url.set_path(&path);
        }

        // Repos can generally be accessed with or without `.git` extension.
        if let Some((prefix, suffix)) = url.path().rsplit_once('@') {
            // Ex) `git+https://github.com/pypa/sample-namespace-packages.git@2.0.0`
            let needs_chopping = std::path::Path::new(prefix)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("git"));
            if needs_chopping {
                let prefix = &prefix[..prefix.len() - 4];
                url.set_path(&format!("{prefix}@{suffix}"));
            }
        } else {
            // Ex) `git+https://github.com/pypa/sample-namespace-packages.git`
            let needs_chopping = std::path::Path::new(url.path())
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("git"));
            if needs_chopping {
                let last = {
                    let last = url.path_segments().unwrap().next_back().unwrap();
                    last[..last.len() - 4].to_owned()
                };
                url.path_segments_mut().unwrap().pop().push(&last);
            }
        }

        Self(url)
    }

    pub fn parse(url: &str) -> Result<Self, url::ParseError> {
        Ok(Self::new(&Url::parse(url)?))
    }
}


/// Like [`CanonicalUrl`], but attempts to represent an underlying source repository, abstracting
/// away details like the specific commit or branch, or the subdirectory to build within the
/// repository.
///
/// For example, `https://github.com/pypa/package.git#subdirectory=pkg_a` and
/// `https://github.com/pypa/package.git#subdirectory=pkg_b` would map to different
/// [`CanonicalUrl`] values, but the same [`RepositoryUrl`], since they map to the same
/// resource.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct RepositoryUrl(Url);

impl RepositoryUrl {
    pub fn new(url: &Url) -> Self {
        let mut url = CanonicalUrl::new(url).0;

        // If a Git URL ends in a reference (like a branch, tag, or commit), remove it.
        let mut url = if url.scheme().starts_with("git+") {
            if let Some(prefix) = url
                .path()
                .rsplit_once('@')
                .map(|(prefix, _suffix)| prefix.to_string())
            {
                url.set_path(&prefix);
            }

            // Remove the `git+` prefix.
            let url_as_str = &url.as_str()[4..];
            Url::parse(url_as_str).expect("url should be valid")
        } else {
            url
        };

        // Drop any fragments and query parameters.
        url.set_fragment(None);
        url.set_query(None);

        Self(url)
    }

    pub fn parse(url: &str) -> Result<Self, url::ParseError> {
        Ok(Self::new(&Url::parse(url)?))
    }

    /// Return the underlying [`Url`] of this repository.
    pub fn into_url(self) -> Url {
        self.into()
    }
}

impl From<RepositoryUrl> for Url {
    fn from(url: RepositoryUrl) -> Self {
        url.0
    }
}

fn need_reinstall(
    installed: &InstalledDist,
    locked: &PypiPackageData,
    lock_file_dir: &Path,
) -> Result<ValidateCurrentInstall, Box<dyn Error>> {
    // Check if the installed version is the same as the required version
    match installed {
        InstalledDist::Registry(reg) => {
            let specifier = to_uv_version(&locked.version)?;

            if reg.version != specifier {
                return Ok(ValidateCurrentInstall::Reinstall);
            }
        }

        // For installed distributions check the direct_url.json to check if a re-install is needed
        InstalledDist::Url(direct_url) => {
            let direct_url_json = match InstalledDist::direct_url(&direct_url.path) {
                Ok(Some(direct_url)) => direct_url,
                Ok(None) => {
                    return Ok(ValidateCurrentInstall::Reinstall);
                }
                Err(_) => {
                    return Ok(ValidateCurrentInstall::Reinstall);
                }
            };

            match direct_url_json {
                uv_pypi_types::DirectUrl::LocalDirectory { url, dir_info } => {
                    // Recreate file url
                    let result = Url::parse(&url);
                    match result {
                        Ok(url) => {
                            // Convert the locked location, which can be a path or a url, to a url
                            let locked_url = match &locked.location {
                                // Fine if it is already a url
                                UrlOrPath::Url(url) => url.clone(),
                                // Do some path mangling if it is actually a path to get it into a url
                                UrlOrPath::Path(path) => {
                                    let path = PathBuf::from(path.as_str());
                                    // Because the path we are comparing to is absolute we need to convert
                                    let path = if path.is_absolute() {
                                        path
                                    } else {
                                        // Relative paths will be relative to the lock file directory
                                        lock_file_dir.join(path)
                                    };
                                    // Okay, now convert to a file path, if we cant do that we need to re-install
                                    match Url::from_file_path(path.clone()) {
                                        Ok(url) => url,
                                        Err(_) => {
                                            return Ok(ValidateCurrentInstall::Reinstall);
                                        }
                                    }
                                }
                            };

                            // Check if the urls are different
                            if url == locked_url {
                                // Okay so these are the same, but we need to check if the cache is newer
                                // than the source directory
                                if !check_url_freshness(&url, installed)? {
                                    return Ok(ValidateCurrentInstall::Reinstall);
                                }
                            } else {
                                return Ok(ValidateCurrentInstall::Reinstall);
                            }
                        }
                        Err(_) => {
                            return Ok(ValidateCurrentInstall::Reinstall);
                        }
                    }
                    // If editable status changed also re-install
                    if dir_info.editable.unwrap_or_default() != locked.editable {
                        return Ok(ValidateCurrentInstall::Reinstall);
                    }
                }
                uv_pypi_types::DirectUrl::ArchiveUrl {
                    url,
                    // Don't think anything ever fills this?
                    archive_info: _,
                    // Subdirectory is either in the url or not supported
                    subdirectory: _,
                } => {
                    let locked_url = match &locked.location {
                        // Remove `direct+` scheme if it is there so we can compare the required to
                        // the installed url
                        UrlOrPath::Url(url) => strip_direct_scheme(url),
                        UrlOrPath::Path(_path) => {
                            return Ok(ValidateCurrentInstall::Reinstall)
                        }
                    };

                    // Try to parse both urls
                    let installed_url = url.parse::<Url>();

                    // Same here
                    let installed_url = if let Ok(installed_url) = installed_url {
                        installed_url
                    } else {
                        return Ok(ValidateCurrentInstall::Reinstall);
                    };

                    if locked_url.as_ref() == &installed_url {
                        // Check cache freshness
                        if !check_url_freshness(&locked_url, installed)? {
                            return Ok(ValidateCurrentInstall::Reinstall);
                        }
                    } else {
                        return Ok(ValidateCurrentInstall::Reinstall);
                    }
                }
                uv_pypi_types::DirectUrl::VcsUrl {
                    url,
                    vcs_info,
                    subdirectory: _,
                } => {
                    // Check if the installed git url is the same as the locked git url
                    // if this fails, it should be an error, because then installed url is not a git url
                    let installed_git_url =
                        uv_pypi_types::ParsedGitUrl::try_from(Url::parse(url.as_str())?)?;

                    // Try to parse the locked git url, this can be any url, so this may fail
                    // in practice it always seems to succeed, even with a non-git url
                    let locked_git_url = match &locked.location {
                        UrlOrPath::Url(url) => {
                            // is it a git url?
                            if LockedGitUrl::is_locked_git_url(url) {
                                let locked_git_url = LockedGitUrl::new(url.clone());
                                to_parsed_git_url(&locked_git_url)
                            } else {
                                // it is not a git url, so we fallback to use the url as is
                                uv_pypi_types::ParsedGitUrl::try_from(url.clone())
                                    .map_err(|_| "Problem parsing git url".into())
                            }
                        }
                        UrlOrPath::Path(_path) => {
                            // Previously
                            return Ok(ValidateCurrentInstall::Reinstall);
                        }
                    };
                    match locked_git_url {
                        Ok(locked_git_url) => {
                            // Check the repository base url with the locked url
                            let installed_repository_url =
                                RepositoryUrl::new(installed_git_url.url.repository());
                            if locked_git_url.url.repository()
                                != &installed_repository_url.into_url()
                            {
                                // This happens when this is not a git url
                                return Ok(ValidateCurrentInstall::Reinstall);
                            }
                            if vcs_info.requested_revision
                                != locked_git_url
                                    .url
                                    .reference()
                                    .as_str()
                                    .map(|s| s.to_string())
                            {
                                // The commit id is different, we need to reinstall
                                return Ok(ValidateCurrentInstall::Reinstall);
                            }
                        }
                        Err(_) => {
                            return Ok(ValidateCurrentInstall::Reinstall);
                        }
                    }
                }
            }
        }
        // Figure out what to do with these
        InstalledDist::EggInfoFile(installed_egg) => {
            tracing::warn!(
                "egg-info files are not supported yet, skipping: {}",
                installed_egg.name
            );
        }
        InstalledDist::EggInfoDirectory(installed_egg_dir) => {
            tracing::warn!(
                "egg-info directories are not supported yet, skipping: {}",
                installed_egg_dir.name
            );
        }
        InstalledDist::LegacyEditable(egg_link) => {
            tracing::warn!(
                ".egg-link pointers are not supported yet, skipping: {}",
                egg_link.name
            );
        }
    };

    // Do some extra checks if the version is the same
    let metadata = match installed.metadata() {
        Ok(metadata) => metadata,
        Err(_err) => {
            // Can't be sure lets reinstall
            return Ok(ValidateCurrentInstall::Reinstall);
        }
    };

    if let Some(requires_python) = metadata.requires_python {
        // If the installed package requires a different requires python version of the locked package,
        // or if one of them is `Some` and the other is `None`.
        match &locked.requires_python {
            Some(locked_requires_python) => {
                if requires_python.to_string() != locked_requires_python.to_string() {
                    return Ok(ValidateCurrentInstall::Reinstall);
                }
            }
            None => {
                return Ok(ValidateCurrentInstall::Reinstall);
            }
        }
    } else if let Some(_requires_python) = &locked.requires_python {
        return Ok(ValidateCurrentInstall::Reinstall);
    }

    Ok(ValidateCurrentInstall::Keep)
}

const UV_INSTALLER: &str = "uv-dof";

impl InstallPlanner {
    pub fn new(uv_cache: Cache, lock_file_dir: impl AsRef<Path>) -> Self {
        Self {
            uv_cache,
            lock_file_dir: lock_file_dir.as_ref().to_path_buf(),
        }
    }

    /// Decide if we need to get the distribution from the local cache or the registry
    /// this method will add the distribution to the local or remote vector,
    /// depending on whether the version is stale, available locally or not
    fn decide_installation_source<'a>(
        &self,
        name: &'a uv_normalize::PackageName,
        required_pkg: &PypiPackageData,
        local: &mut Vec<CachedDist>,
        remote: &mut Vec<Dist>,
        dist_cache: &mut impl CachedDistProvider<'a>,
    ) -> Result<(), Box<dyn Error>> {
        // Okay so we need to re-install the package
        // let's see if we need the remote or local version

        // First, check if we need to revalidate the package
        // then we should get it from the remote
        if self.uv_cache.must_revalidate(name) {
            remote.push(convert_to_dist(required_pkg, &self.lock_file_dir)?);
            return Ok(());
        }
        let uv_version = to_uv_version(&required_pkg.version)?;
        // If it is not stale its either in the registry cache or not
        let cached = dist_cache.get_cached_dist(name, uv_version);
        // If we have it in the cache we can use that
        if let Some(distribution) = cached {
            local.push(CachedDist::Registry(distribution));
        // If we don't have it in the cache we need to download it
        } else {
            remote.push(convert_to_dist(required_pkg, &self.lock_file_dir)?);
        }
        Ok(())
    }


    /// Figure out what we can link from the cache locally
    /// and what we need to download from the registry.
    /// Also determine what we need to remove.
    ///
    /// All the 'a lifetimes are to to make sure that the names provided to the CachedDistProvider
    /// are valid for the lifetime of the CachedDistProvider and what is passed to the method
    pub fn plan<'a, Installed: InstalledDistProvider<'a>, Cached: CachedDistProvider<'a> + 'a>(
        &self,
        site_packages: &'a Installed,
        mut dist_cache: Cached,
        required_pkgs: &'a HashMap<uv_normalize::PackageName, &PypiPackageData>,
    ) -> Result<InstallPlan, Box<dyn Error>> {
        // Packages to be removed
        let mut extraneous = vec![];
        // Packages to be installed directly from the cache
        let mut local = vec![];
        // Try to install from the registry or direct url or w/e
        let mut remote = vec![];
        // Packages that need to be reinstalled
        // i.e. need to be removed before being installed
        let mut reinstalls = vec![];

        // Will contain the packages that have been previously installed
        // and a decision has been made what to do with them
        let mut prev_installed_packages = HashSet::new();

        // Walk over all installed packages and check if they are required
        for dist in site_packages.iter() {

            println!("Found installed pypi package: {:?}", dist.name());

            // Check if we require the package to be installed
            let pkg = required_pkgs.get(dist.name());
            // Get the installer name
            let installer = dist
                .installer()
                // Empty string if no installer or any other error
                .map_or(String::new(), |f| f.unwrap_or_default());

            match pkg {
                Some(required_pkg) => {
                    // Add to the list of previously installed packages
                    prev_installed_packages.insert(dist.name());
                    // Check if we need this package installed but it is not currently installed by us
                    if installer != UV_INSTALLER {
                        // We are managing the package but something else has installed a version
                        // let's re-install to make sure that we have the **correct** version
                        reinstalls.push(dist.clone());
                    } else {
                        println!("huh?");
                        // Check if we need to reinstall
                        match need_reinstall(dist, required_pkg, &self.lock_file_dir)? {
                            ValidateCurrentInstall::Keep => {
                                // No need to reinstall
                                continue;
                            }
                            ValidateCurrentInstall::Reinstall => {
                                reinstalls.push(dist.clone());
                            }
                        }
                    }

                    // Okay so we need to re-install the package
                    // let's see if we need the remote or local version
                    self.decide_installation_source(
                        dist.name(),
                        required_pkg,
                        &mut local,
                        &mut remote,
                        &mut dist_cache,
                    )?;
                }
                // Second case we are not managing the package
                None if installer != UV_INSTALLER => {
                    // Ignore packages that we are not managed by us
                    continue;
                }
                // Third case we *are* managing the package but it is no longer required
                None => {
                    // Add to the extraneous list
                    // as we do manage it but have no need for it
                    extraneous.push(dist.clone());
                }
            }
        }

        // Now we need to check if we have any packages left in the required_map
        for (name, pkg) in required_pkgs
            .iter()
            // Only check the packages that have not been previously installed
            .filter(|(name, _)| !prev_installed_packages.contains(name))
        {
            // Decide if we need to get the distribution from the local cache or the registry
            self.decide_installation_source(
                name,
                pkg,
                &mut local,
                &mut remote,
                &mut dist_cache,
            )?;
        }

        Ok(InstallPlan {
            local,
            remote,
            reinstalls,
            extraneous,
        })
    }

}

fn get_arch_tags(platform: &Platform) -> Result<uv_platform_tags::Arch, Box<dyn Error>> {
    match platform.arch() {
        None => unreachable!("every platform we support has an arch"),
        Some(Arch::X86) => Ok(uv_platform_tags::Arch::X86),
        Some(Arch::X86_64) => Ok(uv_platform_tags::Arch::X86_64),
        Some(Arch::Aarch64 | Arch::Arm64) => Ok(uv_platform_tags::Arch::Aarch64),
        Some(Arch::ArmV7l) => Ok(uv_platform_tags::Arch::Armv7L),
        Some(Arch::Ppc64le) => Ok(uv_platform_tags::Arch::Powerpc64Le),
        Some(Arch::Ppc64) => Ok(uv_platform_tags::Arch::Powerpc64),
        Some(Arch::S390X) => Ok(uv_platform_tags::Arch::S390X),
        Some(unsupported_arch) => {
            panic!("unsupported arch for pypi packages '{unsupported_arch}'")
        }
    }
}

fn rattler_platform_to_uv_platform(platform: Platform) -> Result<uv_platform_tags::Platform, Box<dyn Error>> {
    if platform.is_linux() {
        // Taken from pixi_default_versions
        let os: uv_platform_tags::Os = uv_platform_tags::Os::Manylinux{major: 2, minor: 28};
        let arch = get_arch_tags(&platform)?;
        Ok(uv_platform_tags::Platform::new(os, arch))
    } else if platform.is_windows() {
        Err("Unsupported platform".into())
    } else if platform.is_osx() {
        Err("Unsupported platform".into())
    } else {
        Err("Unsupported platform".into())
    }
}

/// Convert locked indexes to IndexLocations
fn locked_indexes_to_index_locations(
    indexes: &rattler_lock::PypiIndexes,
    base_path: &Path,
) -> Result<IndexLocations, Box<dyn Error>> {
    // Check if the base path is absolute
    // Otherwise uv might panic
    if !base_path.is_absolute() {
        return Err("Base path is not absolute".into())
    }

    let index = indexes
        .indexes
        .first()
        .cloned()
        .map(VerbatimUrl::from_url)
        .map(IndexUrl::from)
        .map(Index::from_index_url)
        .into_iter();
    let extra_indexes = indexes
        .indexes
        .iter()
        .skip(1)
        .cloned()
        .map(VerbatimUrl::from_url)
        .map(IndexUrl::from)
        .map(Index::from_extra_index_url);
    let flat_indexes = indexes
        .find_links
        .iter()
        .map(|url| match url {
            rattler_lock::FindLinksUrlOrPath::Path(relative) => {
                VerbatimUrl::from_path(relative, base_path)
                    .map_err(|_| -> Box<dyn Error> { "Couldn't convert path to flat index location.".into() })
            }
            rattler_lock::FindLinksUrlOrPath::Url(url) => Ok(VerbatimUrl::from_url(url.clone())),
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(IndexUrl::from)
        .map(Index::from_find_links)
        .collect();

    // we don't have support for an explicit `no_index` field in the `PypiIndexes`
    // so we only set it if you want to use flat indexes only
    let indexes: Vec<_> = index.chain(extra_indexes).collect();
    let flat_index: Vec<_> = flat_indexes;
    let no_index = indexes.is_empty() && !flat_index.is_empty();
    Ok(IndexLocations::new(indexes, flat_index, no_index))
}

/// Install the given packages into the prefix.
///
/// If the packages exist in the cache, those will be used. Otherwise, download the requested
/// versions and install all into the prefix.
///
/// Modified from install_pypi::update_python_distributions
async fn install_pypi_packages(
    prefix: PathBuf,
    packages: Vec<LockedPackage>,
    environment_variables: &HashMap<String, String>
) -> Result<(), Box<dyn Error>> {
    // Hard code this for now, otherwise we depend on a lot of pixi code
    let tags = Tags::from_env(
        &rattler_platform_to_uv_platform(Platform::Linux64)?,
        (3, 12),
        "cpython",
        (3, 12),
        true,
        false,
    )?;

    println!("Tags acquired");

    let lockfile_dir = dirs::cache_dir()
        .ok_or("Couldn't find lockfile directory")?
        .join("dof-cache");

    // Get or create the local uv cache
    let uv_cache_dir = dirs::cache_dir()
        .ok_or("Couldn't find uv cache directory")?
        .join("uv-cache");

    if !uv_cache_dir.exists() {
        fs_err::create_dir_all(&uv_cache_dir)
            .map_err(|_| "Failed to create uv cache directory.")?;
    }

    let uv_cache = Cache::from_path(&uv_cache_dir);
    // Get the python interpreter for the prefix
    let python_location = prefix.join("bin/python");

    println!("Lockfile directory: {}", lockfile_dir.to_string_lossy());
    println!("uv cache dir: {}", uv_cache_dir.to_string_lossy());
    println!("python location {:?}", python_location);

    let interpreter = Interpreter::query(python_location, &uv_cache)?;
    println!(
        "Installing into interpreter {} at {}", interpreter.key(), interpreter.sys_prefix().display()
    );

    let venv = PythonEnvironment::from_interpreter(interpreter);
    println!(
        "venv.root {}", venv.root().to_string_lossy()
    );

    // uv registry settings
    let config_settings = ConfigSettings::default();
    let client = Client::new();
    let keyring_provider = KeyringProviderType::Disabled;
    let pypi_indexes: Option<&PypiIndexes> = None;
    let index_locations = pypi_indexes
        .map(|indexes| locked_indexes_to_index_locations(indexes, prefix.as_path()))
        .unwrap_or_else(|| Ok(IndexLocations::default()))?;
    let build_options = BuildOptions::new(
        uv_configuration::NoBinary::default(),
        uv_configuration::NoBuild::None,
    );

    // This is used to find wheels that are available from the registry
    let registry_index = RegistryWheelIndex::new(
        &uv_cache,
        &tags,
        &index_locations,
        &HashStrategy::None,
        &config_settings,
    );
    let registry_client = Arc::new(
        RegistryClientBuilder::new(uv_cache.clone())
            .client(client.clone())
            // Allow connectsion to arbitrary insecure servers (e.g. localhost:8000) as registries
            // .allow_insecure_host(uv_context.allow_insecure_host.clone())
            .index_urls(index_locations.index_urls())
            .keyring(keyring_provider)
            .connectivity(Connectivity::Online)
            .build(),
    );
    // Resolve the flat indexes from `--find-links`.
    let flat_index = {
        let client = FlatIndexClient::new(&registry_client, &uv_cache);
        let indexes = index_locations.flat_indexes().map(|index| index.url());
        let entries = client.fetch(indexes).await?;
        FlatIndex::from_entries(
            entries,
            Some(&tags),
            &uv_types::HashStrategy::None,
            &build_options,
        )
    };

    println!("Setting up BuildDispatch!");

    let concurrency = Concurrency::default();
    let dep_metadata = DependencyMetadata::default();
    let constraints = Constraints::default();
    let in_flight = InFlight::default();
    let shared_state = SharedState::default();
    let build_dispatch = BuildDispatch::new(
        &registry_client,
        &uv_cache,
        constraints,
        venv.interpreter(),
        &index_locations,
        &flat_index,
        &dep_metadata,
        shared_state,
        IndexStrategy::default(),
        &config_settings,
        BuildIsolation::default(),
        LinkMode::default(),
        &build_options,
        &HashStrategy::None, // This is always set by default in pixi when generating the UvResolutionContext from a workspace
        None,
        SourceStrategy::Disabled,
        concurrency,
        PreviewMode::Disabled,
    )
    // ! Important this passes any CONDA activation to the uv build process
    .with_build_extra_env_vars(environment_variables.iter());


    let _lock = venv.lock().await?;

    // Find out what packages are already installed
    let site_packages = SitePackages::from_environment(&venv)
        .expect("could not create site-packages");

    // Warn the user about conda packages that will be filtered out
    let conda_packages: Vec<&CondaPackageData> = packages
        .iter()
        .filter_map(|pkg| {
            pkg.as_conda()
        })
        .collect();

    println!("Conda packages passed in: {}", conda_packages.len());

    // Create a map of the required packages
    let required_map: HashMap<PackageName, &PypiPackageData> =
        packages
            .iter()
            .filter_map(|pkg| pkg.as_pypi())
            .map(|(pkg, _)| {
                let uv_name = PackageName::new(pkg.name.to_string())
                    .expect("should be correct");
                (uv_name, pkg)
            })
            .collect();


    // Determine the currently installed conda packages.
    //
    // Only used to figure out which wheels will clobber conda packages.
    //
    // let installed_packages = find_installed_packages(prefix.as_path())
    //     .map_err(|_| PyErr::new::<PyValueError, _>(
    //         "Cannot determine installed packages in the given environment."
    //     ))?;


    println!("PyPI packages passed in: {}", required_map.len());

    let planner = InstallPlanner::new(uv_cache.clone(), lockfile_dir);

    let InstallPlan {
        local,
        remote,
        reinstalls,
        extraneous,
    } = planner.plan(
        &site_packages,
        registry_index,
        &required_map,
    )?;

    println!("Install plan generated.\n  local: {:?}\n  remote: {:?}\n  reinstalls: {:?}\n  extraneous: {:?}", local, remote, reinstalls, extraneous);

    let remote_dists = acquire_missing_distributions(
        remote,
        Arc::clone(&registry_client),
        &index_locations,
        &build_dispatch,
        &concurrency,
        &uv_cache,
        &tags,
        &build_options,
        &in_flight,
    ).await?;

    let _ = remove_unncessary_packages(extraneous, reinstalls).await;

    // Install the resolved distributions.
    // At this point we have all the wheels we need to install available to link locally
    let local_dists = local.iter().map(|d| d.clone());
    let all_dists = remote_dists
        .into_iter()
        .chain(local_dists)
        .collect::<Vec<_>>();


    if !all_dists.is_empty() {
        Installer::new(&venv)
            .with_link_mode(LinkMode::default())
            .with_installer_name(Some(UV_INSTALLER.to_string()))
            // .with_reporter(UvReporter::new_arc(options))
            .install(all_dists.clone())
            .await
            .expect("should be able to install all distributions");
    }
    println!("Installation complete.");


    Ok(())
}

// Download, build, and unzip any missing distributions.
#[allow(clippy::too_many_arguments)]
async fn acquire_missing_distributions<'a>(
    remote: Vec<Dist>,
    registry_client: Arc<RegistryClient>,
    index_locations: &IndexLocations,
    build_dispatch: &'a BuildDispatch<'a>,
    concurrency: &'a Concurrency,
    uv_cache: &'a Cache,
    tags: &'a Tags,
    build_options: &'a BuildOptions,
    in_flight: &'a InFlight,
) -> Result<Vec<CachedDist>, Box<dyn Error>> {
    if remote.is_empty() {
        Ok(Vec::new())
    } else {
        let distribution_database = DistributionDatabase::new(
            registry_client.as_ref(),
            build_dispatch,
            concurrency.downloads,
        );

        // Before hitting the network let's make sure the credentials are available to
        // uv
        for url in index_locations.indexes().map(|index| index.url()) {
            let success = uv_git::store_credentials_from_url(url);
            tracing::debug!("Stored credentials for {}: {}", url, success);
        }

        let preparer = Preparer::new(
            uv_cache,
            tags,
            &uv_types::HashStrategy::None,
            build_options,
            distribution_database,
        );

        let resolution = Resolution::default();
        let remote_dists = preparer
            .prepare(
                remote.iter().map(|d| d.clone()).collect(),
                in_flight,
                &resolution,
            )
            .await?;

        Ok(remote_dists)
    }
}

async fn remove_unncessary_packages(
    extraneous: Vec<InstalledDist>,
    reinstalls: Vec<InstalledDist>,
) -> Result<(), Box<dyn Error>> {
    // Remove any unnecessary packages.
    if !extraneous.is_empty() || !reinstalls.is_empty() {
        for dist_info in extraneous.iter().chain(reinstalls.iter()) {
            let summary = match uv_installer::uninstall(dist_info).await {
                Ok(sum) => sum,
                // Get error types from uv_installer
                Err(UninstallError::Uninstall(e))
                    if matches!(e, uv_install_wheel::Error::MissingRecord(_))
                        || matches!(e, uv_install_wheel::Error::MissingTopLevel(_)) =>
                {
                    // If the uninstallation failed, remove the directory manually and continue
                    tracing::debug!("Uninstall failed for {:?} with error: {}", dist_info, e);

                    // Sanity check to avoid calling remove all on a bad path.
                    if dist_info
                        .path()
                        .iter()
                        .any(|segment| Path::new(segment) == Path::new("site-packages"))
                    {
                        tokio::fs::remove_dir_all(dist_info.path()).await?;
                    }

                    continue;
                }
                Err(err) => {
                    return Err(err.into());
                }
            };
            tracing::debug!(
                "Uninstalled {} ({} file{}, {} director{})",
                dist_info.name(),
                summary.file_count,
                if summary.file_count == 1 { "" } else { "s" },
                summary.dir_count,
                if summary.dir_count == 1 { "y" } else { "ies" },
            );
        }
    }
    Ok(())
}

fn extract_locked_package<'py>(
    _py: Python<'py>, obj: &Bound<'py, PyAny>
) -> Result<LockedPackage, Box<dyn Error>> {
    let name: String = obj.getattr("name").and_then(|attr| attr.extract())?;
    let version: String = obj.getattr("pypi_version").and_then(|attr| attr.extract())?;
    let location: String = obj.getattr("location").and_then(|attr| attr.extract())?;
    let editable: bool = obj.getattr("pypi_is_editable").and_then(|attr| attr.extract())?;
    let pypi_requires_dist: Vec<String> = obj.getattr("pypi_requires_dist").and_then(|attr| attr.extract())?;
    let pypi_requires_python: Option<String> = obj.getattr("pypi_requires_python").and_then(|attr| attr.extract())?;


    let requires_dist: Vec<pep508_rs::Requirement> = pypi_requires_dist
        .iter()
        .map(|item| -> pep508_rs::Requirement {
            pep508_rs::Requirement::from_str(item.as_str()).unwrap()
        }).collect::<Vec<_>>();

    let requires_python = pypi_requires_python.and_then(
        |spec| pep440_rs::VersionSpecifiers::from_str(&spec).ok()
    );

    let package_hashes: Bound<'py, PyAny> = obj.getattr("hashes")?;
    let md5: Bound<'py, PyAny> = package_hashes.getattr("md5")?;
    let sha256: Bound<'py, PyAny> = package_hashes.getattr("sha256")?;

    // let hash = PackageHashes::from_hashes(
    //     // Some(Md5Hash::from(md5.extract::<Vec<u8>>()?)),
    //     // sha256.extract()?,
    //     // Some(Md5Hash::from(md5.extract()?)),
    //     // Some(Sha256Hash::from(sha256.extract()?)),
    // );

    // let hash = PackageHashes::Md5Sha256(
    //     md5.extract::<Vec<u8>>()?.into(), sha256.extract()?.into()
    // );

    let hash = None;

    Ok(LockedPackage::Pypi(
        PypiPackageData {
            name: pep508_rs::PackageName::from_str(name.as_str())?,
            version: pep440_rs::Version::from_str(version.as_str())?,
            location: UrlOrPath::from_str(location.as_str())?,
            hash,
            requires_dist,
            requires_python,
            editable,
        },
        PypiPackageEnvironmentData{
            extras: std::collections::BTreeSet::new(),
        },
    ))
}

#[pyfunction]
#[pyo3(signature = (packages, prefix = None))]
fn install_pypi<'py>(py: Python<'py>, packages: &Bound<'py, PyList>, prefix: Option<String>) -> PyResult<()> {
    let py_locked_packages: Vec<Bound<'py, PyAny>> = packages
        .iter()
        .map(|pkg| pkg.getattr("_package"))
        .collect::<PyResult<Vec<Bound<'py, PyAny>>>>()?;

    let target_prefix = prefix
        .or_else(|| std::env::var("CONDA_PREFIX").ok())
        .ok_or_else(|| PyErr::new::<PyValueError, _>(
            "No prefix specified and no CONDA_PREFIX found in the environment. \
            Cannot continue."
        ))?;

    let py_pypi_locked_packages: Vec<&Bound<'py, PyAny>> = py_locked_packages
        .iter()
        .filter(|pkg| {
            pkg
                .getattr("is_pypi")
                .and_then(|attr| attr.extract())
                .unwrap_or(false)
        })
        .collect();

    let rs_locked_packages: Vec<LockedPackage> = py_pypi_locked_packages
        .iter()
        .map(|&pkg| {
            extract_locked_package(py, pkg).unwrap()
        })
        .collect();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let result = runtime.block_on(
        install_pypi_packages(
            PathBuf::from_str(target_prefix.as_str())?,
            rs_locked_packages,
            &HashMap::new(),
        )
    ).map_err(|err| {
        PyValueError::new_err("Error running PyPI install")
    });
    result
}

#[pyfunction]
#[pyo3(signature = (lockfile, prefix = None))]
fn install_lockfile<'py>(py: Python<'py>, lockfile: &Bound<'py, PyAny>, prefix: Option<String>) -> PyResult<()> {
    let py_env: Bound<'py, PyAny> = lockfile.call_method0("default_environment")?;
    let py_platform: Bound<'py, PyAny> = PyListMethods::get_item(
        py_env.call_method0("platforms")?.downcast()?,
        0,
    )?;
    let py_packages: Bound<'py, PyAny> = py_env.call_method1("packages", (py_platform,))?;

    install_pypi(py, py_packages.downcast()?, prefix)
}

/// A Python module implemented in Rust.
#[pymodule(name="_dof")]
fn dof(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(install_pypi, m)?)?;
    m.add_function(wrap_pyfunction!(install_lockfile, m)?)?;
    Ok(())
}
