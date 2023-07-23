use super::types::{CommunityView, ListingType, PostView, SortType};
use serde::{Deserialize, Serialize};

/// Includes an authentication parameter in the request
#[derive(Debug, Serialize)]
pub(crate) struct WithAuth<'a, T> {
    #[serde(flatten)]
    pub payload: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<&'a str>,
}

/// A simple community response
#[derive(Debug, Deserialize)]
pub struct CommunityResponse {
    pub community_view: CommunityView,
    pub discussion_languages: Vec<i32>,
}

/// Follow / subscribe to a community
#[derive(Debug, Serialize)]
pub struct FollowCommunity {
    pub community_id: i32,
    pub follow: bool,
}

/// Get a community. Must provide either an id or a name
#[derive(Debug, Serialize)]
pub struct GetCommunity {
    pub id: i32,
}

/// Get a list of posts
#[derive(Debug, Serialize)]
pub struct GetPosts {
    pub type_: ListingType,
    pub sort: SortType,
    pub community_id: Option<i32>,
    pub page: i32,
    pub limit: i32,
}

/// The post list response
#[derive(Debug, Deserialize)]
pub struct GetPostsResponse {
    pub posts: Vec<PostView>,
}

/// Fetches a list of communities
#[derive(Debug, Serialize)]
pub struct ListCommunities {
    pub type_: ListingType,
    pub sort: SortType,
    pub show_nsfw: bool,
    pub page: i32,
    pub limit: i32,
}

/// The response for listing communities
#[derive(Debug, Deserialize)]
pub struct ListCommunitiesResponse {
    pub communities: Vec<CommunityView>,
}

#[derive(Debug, Serialize)]
pub struct Login<'a> {
    #[serde(rename = "username_or_email")]
    pub username: &'a str,
    pub password: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub jwt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NodeInfoResponse {
    pub version: String,
    pub software: NodeInfoSoftware,
    pub protocols: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct NodeInfoSoftware {
    pub name: String,
    pub version: String,
}

/// Does an apub fetch for an object
#[derive(Debug, Serialize)]
pub struct ResolveObject<'q> {
    /// Can be the full url, or a shortened version like: !fediverse@lemmy.ml
    pub q: &'q str,
}

/// The response of an opub object fetch
#[derive(Debug, Deserialize)]
pub struct ResolveObjectResponse {
    pub community: Option<CommunityView>,
}
