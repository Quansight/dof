use std::sync::LazyLock;
use std::path::{PathBuf, Path};
use std::collections::HashMap;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyList;
use pyo3::prelude::*;
use std::error::Error;

use uv_cache::Cache;
use uv_client::{Connectivity, FlatIndexClient, RegistryClientBuilder};
use uv_configuration::{ConfigSettings, Constraints, IndexStrategy, PreviewMode, RAYON_INITIALIZE};
use uv_dispatch::{BuildDispatch, SharedState};
use uv_distribution::{DistributionDatabase, RegistryWheelIndex};
use uv_distribution_types::{DependencyMetadata, IndexLocations, Name, Resolution, IndexUrl, Index};
use uv_pep508::{InvalidNameError, PackageName, VerbatimUrl, VerbatimUrlError};
use uv_install_wheel::LinkMode;
use uv_installer::{Preparer, SitePackages, UninstallError, Installer};
use uv_normalize::PackageName;
use uv_python::{Interpreter, PythonEnvironment};
use uv_resolver::FlatIndex;
use uv_types::HashStrategy;
use uv_platform_tags::Tags;


use rattler_conda_types::{Platform, PrefixRecord, Arch};
use rattler_lock::{LockedPackage, PypiIndexes, PypiPackageData, PypiPackageEnvironmentData};

fn find_installed_packages(path: &Path) -> Result<Vec<PrefixRecord>, std::io::Error> {
    // Initialize rayon explicitly to avoid implicit initialization.
    LazyLock::force(&RAYON_INITIALIZE);
    PrefixRecord::collect_from_prefix(path)
}

fn get_arch_tags(platform: &Platform) -> Result<uv_platform_tags::Arch, Box<dyn std::error::Error>> {
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
async fn _install_pypi(prefix: PathBuf, packages: Vec<LockedPackage>) -> Result<(), Box<dyn Error>> {
    let pypi_indexes: Option<&PypiIndexes>;

    // Hard code this for now, otherwise we depend on a lot of pixi code
    let tags = Tags::from_env(
        &rattler_platform_to_uv_platform(Platform::Linux64)?,
        (3, 12),
        "cpython",
        (3, 12),
        true,
        false,
    )?;

    let index_locations = pypi_indexes
        .map(|indexes| locked_indexes_to_index_locations(indexes, prefix.as_path()))
        .unwrap_or_else(|| Ok(IndexLocations::default()))?;


    // Get or create the local uv cache
    let uv_cache_dir = dirs::cache_dir()
        .ok_or("Couldn't find uv cache directory")?
        .join("uv-cache");

    if !uv_cache_dir.exists() {
        fs_err::create_dir_all(&uv_cache_dir)
            .map_err(|_| "Failed to create uv cache directory.")?;
    }

    let uv_cache = Cache::from_path(uv_cache_dir);

    // Get the python interpreter for the prefix
    let python_location = prefix.join("bin/python");
    let interpreter = Interpreter::query(&python_location, &uv_cache)?;
    println!(
        "Installing into interpreter {} at {}", interpreter.key(), interpreter.sys_prefix().display()
    );

    let venv = PythonEnvironment::from_interpreter(interpreter);
    let _lock = venv.lock().await?;

    // Find out what packages are already installed
    let site_packages = SitePackages::from_environment(&venv)
        .expect("could not create site-packages");

    let config_settings = ConfigSettings::default();


    // This is used to find wheels that are available from the registry
    let registry_index = RegistryWheelIndex::new(
        &uv_cache,
        &tags,
        &index_locations,
        &HashStrategy::None,
        &config_settings,
    );

    // Warn the user about conda packages that will be filtered out
    let conda_packages = packages
        .iter()
        .filter_map(|pkg| pkg.as_conda())
        .collect();
    println!("Conda packages passed in! Ignoring: {:?}", conda_packages);

    // Create a map of the required packages
    let required_map: HashMap<PackageName, &PypiPackageData> =
        packages
            .iter()
            .filter_map(|pkg| pkg.as_pypi())
            .map(|(pkg, _)| {
                let uv_name = uv_normalize::PackageName::new(pkg.name.to_string())
                    .expect("should be correct");
                (uv_name, pkg)
            })
            .collect();


    // Determine the currently installed conda packages.
    let installed_packages = find_installed_packages(prefix.as_path())
        .map_err(|_| PyErr::new::<PyValueError, _>(
            "Cannot determine installed packages in the given environment."
        ))?;

    let PixiInstallPlan {
        local,
        remote,
        reinstalls,
        extraneous,
    } = InstallPlanner::new(uv_context.cache.clone(), lock_file_dir).plan(
        &site_packages,
        registry_index,
        &required_map,
    )?;


    // Install the resolved distributions.
    // At this point we have all the wheels we need to install available to link locally
    let local_dists = local.iter().map(|(d, _)| d.clone());
    let all_dists = remote_dists
        .into_iter()
        .chain(local_dists)
        .collect::<Vec<_>>();


    if !all_dists.is_empty() {
        let start = std::time::Instant::now();
        Installer::new(&venv)
            .with_link_mode(LinkMode::default())
            .with_installer_name(Some("uv-dof".to_string()))
            // .with_reporter(UvReporter::new_arc(options))
            .install(all_dists.clone())
            .await
            .expect("should be able to install all distributions");
    }
    println!("Installation complete.");


    Ok(())
}


#[pyfunction]
#[pyo3(signature = (packages, prefix = None))]
fn install_pypi<'py>(_py: Python<'py>, packages: &Bound<'py, PyList>, prefix: Option<String>) -> PyResult<()> {
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

    println!("Prefix: {}", target_prefix);
    println!("Packages: {:?}", py_locked_packages);
    println!("PyPI packages: {:?}", py_pypi_locked_packages);

    _install_pypi(prefix, packages);

    // let result = _install_pypi(prefix, packages).or_else(|_| PyErr::new::<PyValueError, _>(
    //     "Error install pypi packages; cannot continue."
    // ));
    Ok(())
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
fn foo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(install_pypi, m)?)?;
    m.add_function(wrap_pyfunction!(install_lockfile, m)?)?;
    Ok(())
}
