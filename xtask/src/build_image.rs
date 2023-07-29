use crate::util;
use clap::{builder::PossibleValuesParser, Parser};
use eyre::{eyre, WrapErr};
use itertools::Itertools;
use owo_colors::OwoColorize;
use serde::Deserialize;
use std::{collections::HashMap, iter, process::Command};

#[derive(Debug, Parser)]
pub struct Args {
    /// The path template for copying the binary into the image
    ///
    /// Use {{target}} as a placeholder for the target value. For example,
    /// `./target/{{target}}/release/moco` could be used for building the locally.
    #[arg(long, env = "BINARY_PATH")]
    binary_path: String,
    /// A list of targets to build the multi-platform image for
    #[arg(long = "target", env = "TARGETS", value_delimiter = ',')]
    targets: Vec<String>,
    /// The name of the manifest to build
    #[arg(long, env = "MANIFEST")]
    manifest: String,
    /// The transport to use for pushing the built image to the registry
    #[arg(
        long,
        default_value = "docker", 
        env = "TRANSPORT", 
        value_parser = PossibleValuesParser::new(["docker", "oci"]),
    )]
    transport: String,
    /// The final image tags
    ///
    /// These must include the full image and tag. For example ghcr.io/akrantz01/moco:main
    #[arg(long = "tag", env = "TAGS", value_delimiter = ',')]
    tags: Vec<String>,
    /// Labels to apply to the image, in the format name=value
    #[arg(long = "label", env = "LABELS", value_delimiter = ',')]
    labels: Vec<String>,
    /// JSON metadata from the docker/metadata-action
    ///
    /// Must be a JSON object containing a `tags` key that is a list of strings and a `labels` key
    /// that is an object mapping from a string to a string.
    #[arg(long, env = "DOCKER_METADATA_OUTPUT_JSON")]
    docker_metadata: Option<String>,
}

pub fn run(
    Args {
        binary_path,
        targets,
        manifest,
        transport,
        tags,
        labels,
        docker_metadata,
    }: Args,
) -> eyre::Result<()> {
    if targets.is_empty() {
        return Err(eyre!("must specify at least 1 target"));
    }

    let (tags, labels) = construct_tags_and_labels(tags, labels, docker_metadata)?;
    let registry = {
        let tag = tags.first().expect("tags must not be empty");
        let at = tag.find(':');
        at.map(|at| tag.split_at(at).0).unwrap_or(tag)
    };

    println!("{} creating manifest", "INFO".on_bright_cyan());
    exec(buildah().arg("manifest").arg("create").arg(&manifest))
        .wrap_err("failed to create manifest")?;

    for target in targets {
        println!(
            "{} building image for {}",
            "INFO".on_bright_cyan(),
            target.bold()
        );
        let id = build_for_target(&target, &binary_path, &manifest, &labels)
            .wrap_err_with(|| format!("failed to build image for {target}"))?;

        println!(
            "{} pushing image for {} to the registry",
            "INFO".on_bright_cyan(),
            target.bold()
        );
        exec(
            buildah()
                .arg("push")
                .arg(&id)
                .arg(format!("{transport}://{registry}@sha256:{id}")),
        )
        .wrap_err("failed to push image")?;
    }

    for tag in tags {
        println!(
            "{} pushing manifest for {}",
            "INFO".on_bright_cyan(),
            tag.bold()
        );
        exec(
            buildah()
                .arg("manifest")
                .arg("push")
                .arg(&manifest)
                .arg(format!("{transport}://{tag}")),
        )
        .wrap_err_with(|| format!("failed to push manifest to {tag}"))?;
    }

    Ok(())
}

fn build_for_target(
    target: &str,
    binary_path: &str,
    manifest: &str,
    labels: &[String],
) -> eyre::Result<String> {
    let base_image = base_image_for_target(target)
        .ok_or_else(|| eyre!("could not determine base image for platform"))?;
    let platform = platform_for_target(target).ok_or_else(|| {
        eyre!("unknown platform. if this is a new target, add it to `platform_to_target`")
    })?;

    let container = exec(
        buildah()
            .arg("--platform")
            .arg(platform)
            .arg("from")
            .arg(base_image),
    )
    .wrap_err("failed to create builder container")?;

    exec(
        buildah()
            .arg("config")
            .arg("--cmd")
            .arg("[]")
            .arg(&container),
    )
    .wrap_err("failed to set CMD")?;
    exec(
        buildah()
            .arg("config")
            .arg("--entrypoint")
            .arg(r#"[ "/moco" ]"#)
            .arg(&container),
    )
    .wrap_err("failed to set ENTRYPOINT")?;

    let label_args = labels.len() * 2;
    exec(
        buildah()
            .arg("config")
            .args(
                iter::repeat(&String::from("--label"))
                    .interleave_shortest(labels)
                    .take(label_args),
            )
            .arg(&container),
    )
    .wrap_err("failed to set labels")?;

    exec(
        buildah()
            .arg("copy")
            .arg(&container)
            .arg(binary_path.replace("{{target}}", target))
            .arg("/moco"),
    )
    .wrap_err("failed to copy binary into container")?;

    let image = exec(
        buildah()
            .arg("commit")
            .arg("--rm")
            .arg("--manifest")
            .arg(manifest)
            .arg(&container),
    )
    .wrap_err("failed to commit image")?;

    Ok(image)
}

fn buildah() -> Command {
    Command::new("buildah")
}

fn exec(command: &mut Command) -> eyre::Result<String> {
    let output = util::exec(command)?;
    Ok(String::from_utf8(output)?.trim().to_owned())
}

fn base_image_for_target(target: &str) -> Option<&'static str> {
    if target.contains("gnu") {
        Some("docker.io/library/debian:bookworm-slim")
    } else if target.contains("musl") {
        Some("scratch")
    } else {
        None
    }
}

fn platform_for_target(target: &str) -> Option<&'static str> {
    match target {
        "aarch64-unknown-linux-gnu" | "aarch64-unknown-linux-musl" => Some("linux/arm64"),
        "armv7-unknown-linux-gnueabihf" => Some("linux/arm/v7"),
        "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl" => Some("linux/amd64"),
        _ => None,
    }
}

fn construct_tags_and_labels(
    tags: Vec<String>,
    labels: Vec<String>,
    metadata: Option<String>,
) -> eyre::Result<(Vec<String>, Vec<String>)> {
    if (!tags.is_empty() || !labels.is_empty()) && metadata.is_some() {
        return Err(eyre!(
            "--docker-metadata cannot be specified with --tags or --labels"
        ));
    }

    match metadata {
        Some(metadata) => {
            let metadata = serde_json::from_str::<DockerMetadata>(&metadata)
                .wrap_err("invalid docker metadata")?;
            let labels = metadata
                .labels
                .into_iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect();
            Ok((metadata.tags, labels))
        }
        None => {
            if labels.iter().any(|l| !l.contains('=')) {
                Err(eyre!("labels must be formatted as [name]=[value]"))
            } else {
                Ok((tags, labels))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct DockerMetadata {
    tags: Vec<String>,
    labels: HashMap<String, String>,
}
