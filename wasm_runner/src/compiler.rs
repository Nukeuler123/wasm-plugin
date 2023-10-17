//wasm-pack build --release ./ --target web -j1

use std::{fs, path::Path, process::Command};

use tempdir::TempDir;

use crate::{limitation_injector::rewrite, wasm_vm::VMError, Error};

/// Compiles script into webassembly
///
/// This function compiles the code and then uses the limitation injector to put limits on it
pub fn compile(code: String) -> Result<Vec<u8>, Error> {
    let tmp_dir = TempDir::new("wasm-compiler").unwrap();
    let tmp_path = tmp_dir.path();

    //Create the workspace needed to compile in
    create_workspace_skeleton(&tmp_path)?;
    copy_api_to_tmpdir(&tmp_path)?;
    copy_script_skeleton_to_tmpdir(&tmp_path, code)?;
    let output = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "-j1",
            "--target-dir",
            &tmp_path.join("pkg").to_string_lossy().to_string(),
            "--manifest-path",
            &tmp_path.join("Cargo.toml").to_string_lossy().to_string(),
        ])
        .output()?;

    if !output.status.success() {
        return Err(Box::new(VMError::VMCompileFail(String::from_utf8(
            output.stderr,
        )?)));
    }

    let wasm_script = rewrite(&fs::read(
        tmp_path.join("pkg/wasm32-unknown-unknown/release/wasm_script.wasm"),
    )?)?;

    Ok(wasm_script)
}

///Note, you will have to add any new files or directories you make into these functions

/// Creates the base directory structure in the tempdir
fn create_workspace_skeleton(tmp_path: &Path) -> Result<(), Error> {
    fs::write(
        tmp_path.join("Cargo.toml"),
        include_bytes!("../../Cargo.toml.script"),
    )?;
    std::fs::write(
        tmp_path.join("Cargo.lock"),
        include_bytes!("../../Cargo.lock.script"),
    )?;

    fs::create_dir(tmp_path.join("script_api"))?;
    fs::create_dir(tmp_path.join("script_api/src"))?;
    fs::create_dir(tmp_path.join("script"))?;
    fs::create_dir(tmp_path.join("script/src"))?;
    Ok(())
}

/// Copies the lib.rs from the script skeleton and replaces the script.rs in the skeleton with the user's code
fn copy_script_skeleton_to_tmpdir(tmp_path: &Path, code: String) -> Result<(), Error> {
    fs::write(
        tmp_path.join("script/src/lib.rs"),
        include_bytes!("../../wasm_script_skeleton/src/lib.rs"),
    )?;
    fs::write(
        tmp_path.join("script/Cargo.toml"),
        include_bytes!("../../wasm_script_skeleton/Cargo.toml"),
    )?;
    fs::write(tmp_path.join("script/src/script.rs"), code.as_bytes())?;
    Ok(())
}

/// Copies everything from the API directory in the workspace to the new tempdir
fn copy_api_to_tmpdir(tmp_path: &Path) -> Result<(), Error> {
    fs::write(
        tmp_path.join("script_api/src/lib.rs"),
        include_bytes!("../../script_api/src/lib.rs"),
    )?;
    fs::write(
        tmp_path.join("script_api/src/panic.rs"),
        include_bytes!("../../script_api/src/panic.rs"),
    )?;
    fs::write(
        tmp_path.join("script_api/src/script_action.rs"),
        include_bytes!("../../script_api/src/script_action.rs"),
    )?;
    fs::write(
        tmp_path.join("script_api/src/debug.rs"),
        include_bytes!("../../script_api/src/debug.rs"),
    )?;
    fs::write(
        tmp_path.join("script_api/src/data.rs"),
        include_bytes!("../../script_api/src/data.rs"),
    )?;
    fs::write(
        tmp_path.join("script_api/Cargo.toml"),
        include_bytes!("../../script_api/Cargo.toml"),
    )?;
    Ok(())
}
