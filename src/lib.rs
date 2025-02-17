use pyo3::types::PyList;
use std::path::Path;
use pyo3::prelude::*;
use rattler_lock::LockedPackage;



#[pyfunction]
fn install_pypi<'py>(py: Python<'py>, prefix: String, packages: &Bound<'py, PyList>) -> PyResult<()> {
    // Extract Python list elements as Rust types (e.g., strings)
    let packages_rust: Vec<String> = packages
        .iter()
        .map(|pkg| pkg.extract::<String>())
        .collect::<Result<_, _>>()?;

    println!("Prefix: {}", prefix);
    println!("Packages: {:?}", packages_rust);

    // Call your internal logic with Rust types
    // _install_pypi(prefix, packages_rust);
    Ok(())
}

fn _install_pypi(prefix: &Path, packages: Vec<LockedPackage>) -> i32 {

    let pkgs: Vec<&str> = packages.iter().map(|pkg| pkg.name()).collect();
    0
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
