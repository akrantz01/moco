use crate::util;
use eyre::{eyre, WrapErr};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};

pub fn run() -> eyre::Result<()> {
    let targets = load_targets().wrap_err("failed to load targets")?;

    let all = targets
        .iter()
        .map(|g| &g.targets)
        .flatten()
        .collect::<Vec<_>>();
    set_output("targets", all)?;

    let grouped = targets
        .iter()
        .map(|g| FlatGroup {
            name: &g.name,
            targets: g.targets.join(","),
            suffix: g.suffix.as_deref(),
        })
        .collect::<Vec<_>>();
    set_output("matrix", grouped)?;

    Ok(())
}

fn set_output<S: Serialize>(name: &str, value: S) -> io::Result<()> {
    let Ok(output_path) = env::var("GITHUB_OUTPUT") else { return Ok(()) };
    let output_path = Path::new(&output_path);

    let mut output = OpenOptions::new().append(true).open(output_path)?;

    let serialized = serde_json::to_string(&value).expect("must serialize");
    writeln!(output, "{name}={serialized}")
}

fn load_targets() -> eyre::Result<Vec<Group>> {
    let path = util::workspace_root()?.join("targets.json");
    if !path.exists() {
        return Err(eyre!(
            "targets.json must exist at the root of the workspace"
        ));
    }

    let raw = fs::read(path).wrap_err("could not read targets.json")?;
    let targets = serde_json::from_slice(&raw).wrap_err("invalid targets format")?;

    Ok(targets)
}

#[derive(Debug, Serialize)]
struct FlatGroup<'g> {
    name: &'g str,
    targets: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    suffix: Option<&'g str>,
}

#[derive(Debug, Deserialize)]
struct Group {
    name: String,
    targets: Vec<String>,
    suffix: Option<String>,
}
