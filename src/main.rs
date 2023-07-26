use eyre::WrapErr;
use std::{collections::HashSet, sync::Arc};
use tokio::{signal, sync::broadcast};
use tracing::{debug, info};
use url::Url;

mod api;
mod cli;
mod logging;
mod populater;

use api::LemmyApi;
use populater::{FromCommunities, FromPosts};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    if let Err(error) = dotenvy::dotenv() {
        if !error.not_found() {
            return Err(error).wrap_err("invalid .env file");
        }
    }

    let args = cli::parse();
    let ignored = Arc::new(args.ignored.clone().into_iter().collect::<HashSet<_>>());

    logging::init(args.log_level, args.log_targets.as_deref());

    debug!(?args);

    let mut client = LemmyApi::connect(&args.url)
        .await
        .wrap_err("connection to instance failed")?;
    debug!(instance = %args.url, "connected to the local instance");

    client
        .login(&args.username, &args.password)
        .await
        .wrap_err("login failed")?;

    info!(instance = %args.url, "successfully logged in");

    let (stop, _) = broadcast::channel(1);

    let mut tasks = Vec::with_capacity(args.peers.len() * args.sort_methods.len());
    for peer in args.peers {
        let url = Url::parse(&format!("https://{peer}"))
            .wrap_err_with(|| format!("could not build URL for {peer}"))?;

        let peer = LemmyApi::connect(&url)
            .await
            .wrap_err_with(|| format!("cannot connect to peer {peer}"))?;

        for method in &args.sort_methods {
            let communities = tokio::task::spawn(populater::launch::<FromCommunities>(
                client.clone(),
                peer.clone(),
                ignored.clone(),
                *method,
                args.community_count,
                args.community_add_delay,
                args.run_interval,
                stop.subscribe(),
            ));

            let posts = tokio::task::spawn(populater::launch::<FromPosts>(
                client.clone(),
                peer.clone(),
                ignored.clone(),
                *method,
                args.post_count,
                args.community_add_delay,
                args.run_interval,
                stop.subscribe(),
            ));

            tasks.push(communities);
            tasks.push(posts);
        }
    }

    wait_for_terminate().await;

    stop.send(()).wrap_err("populaters stopped unexpectedly")?;

    info!("waiting for populaters to exit...");
    futures::future::join_all(tasks).await;

    info!("successfully shutdown");
    info!("goodbye o/");

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
