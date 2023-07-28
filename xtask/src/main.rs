use clap::Parser;
use eyre::WrapErr;

mod build_image;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    if let Err(error) = dotenvy::dotenv() {
        if !error.not_found() {
            return Err(error).wrap_err("invalid .env file");
        }
    }

    match Command::parse() {
        Command::BuildImage(args) => build_image::run(args),
    }
}

/// A collection of tasks to run
#[derive(Debug, Parser)]
#[command(author, version, about)]
enum Command {
    /// Build the container image
    BuildImage(build_image::Args),
}
