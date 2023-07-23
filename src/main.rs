use eyre::WrapErr;

mod api;
mod cli;

use api::LemmyApi;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    if let Err(error) = dotenvy::dotenv() {
        if !error.not_found() {
            return Err(error).wrap_err("invalid .env file");
        }
    }

    let args = cli::parse();

    let mut client = LemmyApi::connect(&args.api_url).await?;
    client.login(&args.username, &args.password).await?;

    Ok(())
}
