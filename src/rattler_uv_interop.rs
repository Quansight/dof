use std::sync::LazyLock;
use std::borrow::Cow;
use std::error::Error;
use std::path::Path;
use std::str::FromStr;
use miette::IntoDiagnostic;

use url::Url;

use rattler_lock::{PackageHashes, PypiPackageData, UrlOrPath};

use uv_pypi_types::{VerbatimParsedUrl, ParsedUrl, HashDigest, HashAlgorithm};
use uv_cache::{ArchiveTarget, ArchiveTimestamp};
use uv_distribution_filename::{DistExtension, SourceDistExtension, WheelFilename};
use uv_distribution_types::{
    BuiltDist,
    Dist,
    IndexUrl,
    InstalledDist,
    RegistryBuiltDist,
    RegistryBuiltWheel,
    RegistrySourceDist,
    SourceDist,
    UrlString,
};
use uv_normalize::PackageName;

use crate::gitutil::{
    LockedGitUrl,
    to_parsed_git_url,
};

pub static DEFAULT_PYPI_INDEX_URL: LazyLock<Url> =
    LazyLock::new(|| Url::parse("https://pypi.org/simple").unwrap());


/// Converts `pep440_rs::VersionSpecifiers` to `uv_pep440::VersionSpecifiers`
pub fn to_uv_version_specifiers(
    version_specifier: &pep440_rs::VersionSpecifiers,
) -> Result<uv_pep440::VersionSpecifiers, Box<dyn Error>> {
    Ok(uv_pep440::VersionSpecifiers::from_str(&version_specifier.to_string())?)
}

/// Converts our locked data to a file
pub fn locked_data_to_file(
    url: &Url,
    hash: Option<&PackageHashes>,
    filename: &str,
    requires_python: Option<pep440_rs::VersionSpecifiers>,
) -> Result<uv_distribution_types::File, Box<dyn Error>> {
    let url = uv_distribution_types::FileLocation::AbsoluteUrl(UrlString::from(url.clone()));

    // Convert PackageHashes to uv hashes
    let hashes = if let Some(hash) = hash {
        match hash {
            rattler_lock::PackageHashes::Md5(md5) => vec![HashDigest {
                algorithm: HashAlgorithm::Md5,
                digest: format!("{:x}", md5).into(),
            }],
            rattler_lock::PackageHashes::Sha256(sha256) => vec![HashDigest {
                algorithm: HashAlgorithm::Sha256,
                digest: format!("{:x}", sha256).into(),
            }],
            rattler_lock::PackageHashes::Md5Sha256(md5, sha256) => vec![
                HashDigest {
                    algorithm: HashAlgorithm::Md5,
                    digest: format!("{:x}", md5).into(),
                },
                HashDigest {
                    algorithm: HashAlgorithm::Sha256,
                    digest: format!("{:x}", sha256).into(),
                },
            ],
        }
    } else {
        vec![]
    };

    let uv_requires_python = requires_python
        .map(|inside| to_uv_version_specifiers(&inside))
        .transpose()?;

    Ok(uv_distribution_types::File {
        filename: filename.to_string(),
        dist_info_metadata: false,
        hashes,
        requires_python: uv_requires_python,
        upload_time_utc_ms: None,
        yanked: None,
        size: None,
        url,
    })
}

/// Converts `pe508::PackageName` to  `uv_normalize::PackageName`
pub fn to_uv_normalize(
    normalise: &pep508_rs::PackageName,
) -> Result<PackageName, Box<dyn Error>> {
    Ok(PackageName::from_str(normalise.to_string().as_str())?)
}


/// Strip of the `direct` scheme from the url if it is there
pub fn strip_direct_scheme(url: &Url) -> Cow<'_, Url> {
    url.as_ref()
        .strip_prefix("direct+")
        .and_then(|str| Url::from_str(str).ok())
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(url))
}

pub fn to_uv_version(
    version: &pep440_rs::Version,
) -> Result<uv_pep440::Version, Box<dyn Error>> {
    Ok(uv_pep440::Version::from_str(version.to_string().as_str())?)
}

/// Check freshness of a locked url against an installed dist
pub fn check_url_freshness(
    locked_url: &Url,
    installed_dist: &InstalledDist,
) -> miette::Result<bool> {
    if let Ok(archive) = locked_url.to_file_path() {
        // This checks the entrypoints like `pyproject.toml`, `setup.cfg`, and
        // `setup.py` against the METADATA of the installed distribution
        if ArchiveTimestamp::up_to_date_with(&archive, ArchiveTarget::Install(installed_dist))
            .into_diagnostic()?
        {
            tracing::debug!("Requirement already satisfied (and up-to-date): {installed_dist}");
            Ok(true)
        } else {
            tracing::debug!("Requirement already satisfied (but not up-to-date): {installed_dist}");
            Ok(false)
        }
    } else {
        // Otherwise, assume the requirement is up-to-date.
        tracing::debug!("Requirement already satisfied (assumed up-to-date): {installed_dist}");
        Ok(true)
    }
}

/// Check if the url is a direct url
/// Files, git, are direct urls
/// Direct urls to wheels or sdists are prefixed with a `direct` scheme
/// by us when resolving the lock file
pub fn is_direct_url(url_scheme: &str) -> bool {
    url_scheme == "file"
        || url_scheme == "git+http"
        || url_scheme == "git+https"
        || url_scheme == "git+ssh"
        || url_scheme.starts_with("direct")
}

/// Convert from a PypiPackageData to a uv [`distribution_types::Dist`]
pub fn convert_to_dist(
    pkg: &PypiPackageData,
    lock_file_dir: &Path,
) -> Result<Dist, Box<dyn Error>> {
    // Figure out if it is a url from the registry or a direct url
    let dist = match &pkg.location {
        UrlOrPath::Url(url) if is_direct_url(url.scheme()) => {
            let url_without_direct = strip_direct_scheme(url);
            let pkg_name = to_uv_normalize(&pkg.name)?;

            if LockedGitUrl::is_locked_git_url(&url_without_direct) {
                let locked_git_url = LockedGitUrl::new(url_without_direct.clone().into_owned());
                let parsed_git_url = to_parsed_git_url(&locked_git_url)?;

                Dist::from_url(
                    pkg_name,
                    VerbatimParsedUrl {
                        parsed_url: ParsedUrl::Git(parsed_git_url),
                        verbatim: uv_pep508::VerbatimUrl::from(url_without_direct.into_owned()),
                    },
                )?
            } else {
                Dist::from_url(
                    pkg_name,
                    VerbatimParsedUrl {
                        parsed_url: ParsedUrl::try_from(url_without_direct.clone().into_owned())
                            .map_err(Box::new)?,
                        verbatim: uv_pep508::VerbatimUrl::from(url_without_direct.into_owned()),
                    },
                )?
            }
        }
        UrlOrPath::Url(url) => {
            // We consider it to be a registry url
            // Extract last component from registry url
            // should be something like `package-0.1.0-py3-none-any.whl`
            let filename_raw = url
                .path_segments()
                .expect("url should have path segments")
                .last()
                .expect("url should have at least one path segment");

            // Decode the filename to avoid issues with the HTTP coding like `%2B` to `+`
            let filename_decoded =
                percent_encoding::percent_decode_str(filename_raw).decode_utf8_lossy();

            // Now we can convert the locked data to a [`distribution_types::File`]
            // which is essentially the file information for a wheel or sdist
            let file = locked_data_to_file(
                url,
                pkg.hash.as_ref(),
                filename_decoded.as_ref(),
                pkg.requires_python.clone(),
            )?;
            // Recreate the filename from the extracted last component
            // If this errors this is not a valid wheel filename
            // and we should consider it a sdist
            let filename = WheelFilename::from_str(filename_decoded.as_ref());
            if let Ok(filename) = filename {
                Dist::Built(BuiltDist::Registry(RegistryBuiltDist {
                    wheels: vec![RegistryBuiltWheel {
                        filename,
                        file: Box::new(file),
                        // This should be fine because currently it is only used for caching
                        // When upgrading uv and running into problems we would need to sort this
                        // out but it would require adding the indexes to
                        // the lock file
                        index: IndexUrl::Pypi(uv_pep508::VerbatimUrl::from_url(
                            DEFAULT_PYPI_INDEX_URL.clone(),
                        )),
                    }],
                    best_wheel_index: 0,
                    sdist: None,
                }))
            } else {
                let pkg_name = to_uv_normalize(&pkg.name)?;
                let pkg_version = to_uv_version(&pkg.version)?;
                Dist::Source(SourceDist::Registry(RegistrySourceDist {
                    name: pkg_name,
                    version: pkg_version,
                    file: Box::new(file),
                    // This should be fine because currently it is only used for caching
                    index: IndexUrl::Pypi(uv_pep508::VerbatimUrl::from_url(
                        DEFAULT_PYPI_INDEX_URL.clone(),
                    )),
                    // I don't think this really matters for the install
                    wheels: vec![],
                    ext: SourceDistExtension::from_path(Path::new(filename_raw))?,
                }))
            }
        }
        UrlOrPath::Path(path) => {
            let native_path = Path::new(path.as_str());

            if !path.is_absolute() {
                return Err(
                    format!(
                        "Attempting to install package '{}' from a relative path, which is unsupported. Aborting.",
                        pkg.name
                    ).into()
                )
            }
            let abs_path = native_path.to_path_buf();
            let absolute_url = uv_pep508::VerbatimUrl::from_absolute_path(&abs_path)?;

            let pkg_name =
                uv_normalize::PackageName::new(pkg.name.to_string()).expect("should be correct");
            if abs_path.is_dir() {
                Dist::from_directory_url(pkg_name, absolute_url, &abs_path, pkg.editable, false)?
            } else {
                Dist::from_file_url(
                    pkg_name,
                    absolute_url,
                    &abs_path,
                    DistExtension::from_path(&abs_path)?,
                )?
            }
        }
    };

    Ok(dist)
}
