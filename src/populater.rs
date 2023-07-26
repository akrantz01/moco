use crate::api::{Community, FetchError, LemmyApi, ListingType, SortType, SubscribedType};
use rand::Rng;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::broadcast, time};
use tracing::{debug, error, info, info_span, instrument, warn, Instrument, Span};

/// Shared context passed through to the populater
#[derive(Clone)]
pub struct Context {
    local: LemmyApi,
    peer: LemmyApi,
    ignored: Arc<HashSet<String>>,
    add_delay: Duration,
}

/// Create a new context for the populaters
pub fn context(
    local: LemmyApi,
    peer: LemmyApi,
    ignored: Arc<HashSet<String>>,
    add_delay: Duration,
) -> Context {
    Context {
        local,
        peer,
        ignored,
        add_delay,
    }
}

/// Periodically populate the local instance from the peer using the specified source
pub async fn launch<S: CommunitySource>(
    context: Context,
    sort: SortType,
    limit: i32,
    interval: Duration,
    mut stop: broadcast::Receiver<()>,
) {
    let instance = context.peer.instance();
    let kind = S::kind();

    info!(%instance, %kind, ?sort, "populater started");

    sleep_with_jitter(Duration::from_secs(5), 0.5).await;

    loop {
        async {
            if let Err(error) =
                populate::<S>(&context, sort, limit).await
            {
                error!(%instance, %kind, ?sort, error = &error as &(dyn std::error::Error + 'static));
            }

            info!(%instance, %kind, ?sort, "complete, waiting until next interval");
        }
            .instrument(info_span!("populater", %instance, %kind, ?sort))
            .await;

        tokio::select! {
            _ = stop.recv() => break,
            _ = sleep_with_jitter(interval, 0.1) => {},
        }
    }

    info!(%instance, %kind, ?sort, "populater halted");
}

/// Perform the population for the peer
#[instrument(name = "populate", skip_all)]
async fn populate<S: CommunitySource>(
    context: &Context,
    sort: SortType,
    limit: i32,
) -> Result<(), FetchError> {
    let mut processed = HashSet::new();

    let communities = S::fetch(&context.peer, ListingType::Local, sort, limit).await?;
    debug!(found = communities.len());

    for community in communities {
        if let Err(error) = check(&community, &mut processed, context).await {
            error!(id = community.id, actor_id = %community.actor_id, error = &error as &(dyn std::error::Error + 'static));
        }
    }

    Ok(())
}

/// Check the community
#[instrument(name = "check", skip_all, fields(name))]
async fn check(
    community: &Community,
    processed: &mut HashSet<String>,
    Context {
        local,
        peer,
        ignored,
        add_delay,
    }: &Context,
) -> Result<(), FetchError> {
    let instance = community
        .actor_id
        .host_str()
        .expect("community must have a host");
    let name = format!("{}@{instance}", community.name);
    Span::current().record("name", &name);

    if ignored.contains(instance) {
        info!(skipped = true, reason = "in ignore list");
        return Ok(());
    }
    if processed.contains(&name) {
        info!(skipped = true, reason = "already processed community");
        return Ok(());
    }

    if local.get_community(&name).await?.is_some() {
        info!(skipped = true, reason = "already subscribed to community");
        return Ok(());
    }

    let community = match peer.resolve_object(community.actor_id.as_str()).await? {
        Some(c) => c,
        None => {
            warn!("community does not exist on instance");
            return Ok(());
        }
    };

    sleep_with_jitter(*add_delay, 0.25).await;

    info!("following new community");
    local.follow_community(community.community.id).await?;

    processed.insert(name);

    Ok(())
}

/// Sleep the specified amount +/- a 5% jitter
#[instrument(level = "debug", fields(duration = duration.as_secs()))]
async fn sleep_with_jitter(duration: Duration, max_percent: f64) {
    let secs = duration.as_secs_f64();

    let jitter = {
        let mut rng = rand::thread_rng();

        let sign = if rng.gen() { 1f64 } else { -1f64 };
        let percent: f64 = rng.gen_range(0f64..max_percent);
        duration.as_secs_f64() * percent * sign
    };

    debug!(jitter, total = secs + jitter);
    time::sleep(Duration::from_secs_f64(secs + jitter)).await
}

#[async_trait::async_trait]
pub trait CommunitySource {
    fn kind() -> &'static str;

    async fn fetch(
        api: &LemmyApi,
        type_: ListingType,
        sort: SortType,
        limit: i32,
    ) -> Result<Vec<Community>, FetchError>;
}

/// Populate from communities
pub struct FromCommunities;

#[async_trait::async_trait]
impl CommunitySource for FromCommunities {
    fn kind() -> &'static str {
        "communities"
    }

    async fn fetch(
        api: &LemmyApi,
        type_: ListingType,
        sort: SortType,
        limit: i32,
    ) -> Result<Vec<Community>, FetchError> {
        let views = api.list_communities(type_, sort, false, limit).await?;

        let communities = views
            .into_iter()
            .filter(|c| !c.blocked && c.subscribed == SubscribedType::NotSubscribed)
            .map(|c| c.community)
            .collect();
        Ok(communities)
    }
}

/// Populate from posts
pub struct FromPosts;

#[async_trait::async_trait]
impl CommunitySource for FromPosts {
    fn kind() -> &'static str {
        "posts"
    }

    async fn fetch(
        api: &LemmyApi,
        type_: ListingType,
        sort: SortType,
        limit: i32,
    ) -> Result<Vec<Community>, FetchError> {
        let views = api.get_posts(type_, sort, None, limit).await?;

        let communities = views
            .into_iter()
            .filter(|p| !p.creator_blocked && p.subscribed == SubscribedType::NotSubscribed)
            .map(|p| p.community)
            .collect();
        Ok(communities)
    }
}
