use crate::api::{Community, FetchError, LemmyApi, ListingType, SortType, SubscribedType};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::broadcast, time};

/// Periodically populate the local instance from the peer using the specified source
pub async fn launch<S: CommunitySource>(
    local: LemmyApi,
    peer: LemmyApi,
    ignored: Arc<HashSet<String>>,
    sort: SortType,
    limit: i32,
    community_add_delay: Duration,
    interval: Duration,
    mut stop: broadcast::Receiver<()>,
) {
    let instance = peer.instance();
    let kind = S::kind();

    loop {
        if let Err(error) =
            populate::<S>(&local, &peer, &ignored, sort, limit, community_add_delay).await
        {
            println!("ERROR for {instance} ({kind}, {sort:?}): {error}",);
        }

        println!("complete, waiting until next interval");

        tokio::select! {
            _ = stop.recv() => break,
            _ = time::sleep(interval) => {},
        }
    }

    println!("{instance} ({kind} {sort:?}) halted");
}

/// Perform the population for the peer
async fn populate<S: CommunitySource>(
    local: &LemmyApi,
    peer: &LemmyApi,
    ignored: &HashSet<String>,
    sort: SortType,
    limit: i32,
    community_add_delay: Duration,
) -> Result<(), FetchError> {
    let mut searched = HashSet::new();

    let communities = S::fetch(peer, ListingType::Local, sort, limit).await?;
    println!(
        "fetched {} {} by {sort:?} from {}",
        communities.len(),
        S::kind(),
        peer.instance()
    );

    for community in communities {
        let instance = community
            .actor_id
            .host_str()
            .expect("community must have a host");
        let unique_id = format!("{}@{instance}", community.name);

        if ignored.contains(instance) {
            println!("{instance} in ignore list, skipping");
            continue;
        }
        if searched.contains(&unique_id) {
            println!("already processed {unique_id}, skipping");
            continue;
        }

        if local.get_community(&unique_id).await?.is_some() {
            println!("already subscribed to {unique_id}, skipping");
            continue;
        }

        let community = match peer.resolve_object(community.actor_id.as_str()).await? {
            Some(c) => c,
            None => {
                println!("{unique_id} does not exist on {instance}");
                continue;
            }
        };

        tokio::time::sleep(community_add_delay).await;

        println!("following {unique_id}");
        local.follow_community(community.community.id).await?;

        searched.insert(unique_id);
    }

    Ok(())
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
