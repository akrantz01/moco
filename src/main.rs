use eyre::WrapErr;
use tokio::signal;

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

    let mut client = LemmyApi::connect(&args.api_url)
        .await
        .wrap_err("connection to instance failed")?;
    client
        .login(&args.username, &args.password)
        .await
        .wrap_err("login failed")?;

    wait_for_terminate().await;

    Ok(())
}

async fn wait_for_terminate() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install ctrl+c handler")
    };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install terminate signal handler")
            .recv()
            .await
    };
    #[cfg(windows)]
    let terminate = async {
        signal::windows::ctrl_close()
            .expect("failed to install close signal handler")
            .recv()
            .await
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
