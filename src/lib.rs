use std::sync::LazyLock;
use std::path::{PathBuf, Path};
use std::collections::HashMap;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyList;
use pyo3::prelude::*;

use uv_cache::Cache;
use uv_client::{Connectivity, FlatIndexClient, RegistryClientBuilder};
use uv_configuration::{ConfigSettings, Constraints, IndexStrategy, PreviewMode, RAYON_INITIALIZE};
use uv_dispatch::{BuildDispatch, SharedState};
use uv_distribution::{DistributionDatabase, RegistryWheelIndex};
use uv_distribution_types::{DependencyMetadata, IndexLocations, Name, Resolution};
use uv_install_wheel::LinkMode;
use uv_installer::{Preparer, SitePackages, UninstallError, Installer};
use uv_normalize::PackageName;
use uv_python::{Interpreter, PythonEnvironment};
use uv_resolver::FlatIndex;
use uv_types::HashStrategy;
use uv_platform_tags::Tags;


use rattler_conda_types::{Platform, PrefixRecord};
use rattler_lock::{LockedPackage, PypiIndexes, PypiPackageData, PypiPackageEnvironmentData};

fn find_installed_packages(path: &Path) -> Result<Vec<PrefixRecord>, std::io::Error> {
    // Initialize rayon explicitly to avoid implicit initialization.
    LazyLock::force(&RAYON_INITIALIZE);
    PrefixRecord::collect_from_prefix(path)
}



/// Install the given packages into the prefix.
///
/// If the packages exist in the cache, those will be used. Otherwise, download the requested
/// versions and install all into the prefix.
async fn _install_pypi(prefix: PathBuf, packages: Vec<LockedPackage>) -> Result<(), _> {
    let pypi_indexes: Option<&PypiIndexes>;

    let tags: Tags = get_pypi_tags(
        platform,
        system_requirements,
        python_record.package_record(),
    )?;
    let index_locations = pypi_indexes
        .map(|indexes| locked_indexes_to_index_locations(indexes, lock_file_dir))
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


// // Construct an update context and perform the actual update.
// let lock_file_derived_data = UpdateContext::builder(self)
//     .with_package_cache(package_cache)
//     .with_no_install(options.no_install)
//     .with_outdated_environments(outdated)
//     .with_lock_file(lock_file)
//     .with_glob_hash_cache(glob_hash_cache)
//     .finish()
//     .await?
//     .update()
//     .await?;
