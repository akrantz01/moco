use eyre::{eyre, WrapErr};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

static METADATA: OnceCell<CargoMetadata> = OnceCell::new();

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct CargoMetadata {
    workspace_root: PathBuf,
    target_directory: PathBuf,
}

fn metadata() -> eyre::Result<&'static CargoMetadata> {
    METADATA.get_or_try_init(|| {
        let raw = exec(cargo().arg("metadata").arg("--format-version").arg("1"))
            .wrap_err("failed to get workspace metadata")?;
        let metadata = serde_json::from_slice(&raw).wrap_err("invalid workspace metadata")?;

        Ok(metadata)
    })
}

pub fn workspace_root() -> eyre::Result<&'static Path> {
    Ok(metadata()?.workspace_root.as_path())
}

pub fn exec(command: &mut Command) -> eyre::Result<Vec<u8>> {
    println!("::group::{command:?}");
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .wrap_err("failed to execute command")?;
    println!("::endgroup::");

    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(eyre!(
            "command exited with non-zero status ({})",
            output.status
        ))
    }
}

fn cargo() -> Command {
    Command::new("cargo")
}
