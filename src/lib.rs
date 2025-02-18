use pyo3::exceptions::PyValueError;
use pyo3::types::PyList;
use std::path::Path;
use pyo3::prelude::*;
use rattler_lock::LockedPackage;


// In python:
//
// LockFile:
//   list[Environment]
//
// Environment:
//   packages() -> list[LockedPackage]
//
// LockedPackage:
//   _package: PyLockedPackage
//
// PyLockedPackage(LockedPackage):
//   _package: PyLockedPackage  // <-- Must have .is_pypi = True for this to work

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


    println!("Prefix: {}", target_prefix);
    println!("Packages: {:?}", py_locked_packages);

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

fn _install_pypi(prefix: &Path, packages: Vec<LockedPackage>) -> i32 {
    let pkgs: Vec<&str> = packages.iter().map(|pkg| pkg.name()).collect();
    0
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
