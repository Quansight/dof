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
fn install_pypi<'py>(py: Python<'py>, prefix: String, packages: &Bound<'py, PyList>) -> PyResult<()> {
    let py_locked_packages: Vec<Bound<'py, PyAny>> = packages
        .iter()
        .map(|pkg| pkg.getattr("_package"))
        .collect::<PyResult<Vec<_>>>()?;

    println!("Prefix: {}", prefix);
    println!("Packages: {:?}", py_locked_packages);

    Ok(())
}

fn _install_pypi(prefix: &Path, packages: Vec<LockedPackage>) -> i32 {

    let pkgs: Vec<&str> = packages.iter().map(|pkg| pkg.name()).collect();
    0
}

fn install_lockfile<'py>(py: Python<'py>, prefix: String, py_lockfile: &Bound<'py, PyAny>) -> PyResult<()> {
    let py_env: Bound<'py, PyAny> = py_lockfile.call_method0("default_environment")?;
    let py_platform: Bound<'py, PyAny> = PyListMethods::get_item(
        py_env.call_method0("platforms")?.downcast()?,
        0,
    )?;

    let packages: Bound<'py, PyAny> = py_env.call_method1("packages", py_platform)?;

    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule(name="_dof")]
fn foo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(install_pypi, m)?)?;
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
