use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, StatusCode,
};
use url::Url;

mod errors;
mod http;
mod types;

pub use errors::{CommunityError, ConnectError, LoginError, PostError, ResolveError};
use http::{
    CommunityResponse, FollowCommunity, GetCommunity, GetPosts, GetPostsResponse, ListCommunities,
    ListCommunitiesResponse, Login, LoginResponse, NodeInfoResponse, ResolveObject,
    ResolveObjectResponse, WithAuth,
};
pub use types::{
    Community, CommunityView, CommunityViewable, ListingType, PostView, ServerError, SortType,
    SubscribedType,
};

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// A wrapper around the Lemmy API
pub struct LemmyApi {
    base: Url,
    client: Client,
    token: Option<String>,
}

impl LemmyApi {
    /// Connect to a Lemmy instance
    pub async fn connect(base: &Url) -> Result<LemmyApi, ConnectError> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .default_headers({
                let mut map = HeaderMap::new();
                map.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
                map
            })
            .build()?;

        let info = node_info(&client, base).await?;
        if info.version != "2.0" || info.software.name != "lemmy" {
            return Err(ConnectError::NotLemmyInstance);
        }

        // We can't use Vec::contains here since &str doesn't coerce to &String
        if !info
            .protocols
            .iter()
            .any(|protocol| protocol == "activitypub")
        {
            return Err(ConnectError::FederationNotSupported);
        }

        let base = base.join("/api/v3/").expect("url must be valid");

        Ok(LemmyApi {
            client,
            base,
            token: None,
        })
    }

    /// Authenticate with the instance
    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), LoginError> {
        let url = self.base.join("user/login").expect("url must be valid");
        let response = self
            .client
            .post(url)
            .json(&Login { username, password })
            .send()
            .await?;

        if response.status().is_success() {
            let auth = response.json::<LoginResponse>().await?;
            match auth.jwt {
                Some(token) => self.token = Some(token),
                None => return Err(LoginError::IncorrectCredentials),
            }
        } else {
            let error = response.json::<ServerError>().await?;

            return match error.error.as_str() {
                "incorrect_login" => Err(LoginError::IncorrectCredentials),
                "email_not_verified" => Err(LoginError::EmailNotVerified),
                _ => Err(LoginError::ServerError(error)),
            };
        }

        Ok(())
    }

    /// Follow / subscribe to a community
    pub async fn follow_community(&self, id: i32) -> Result<(), CommunityError> {
        let url = self
            .base
            .join("community/follow")
            .expect("url must be valid");
        let payload = FollowCommunity {
            community_id: id,
            follow: true,
        };

        let response = self
            .client
            .post(url)
            .json(&WithAuth::new(payload, &self.token))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else if response.status() == StatusCode::NOT_FOUND {
            Err(CommunityError::NotFound)
        } else {
            let error = response.json().await?;
            Err(CommunityError::ServerError(error))
        }
    }

    /// Get / fetch a community
    pub async fn get_community(&self, id: i32) -> Result<CommunityResponse, CommunityError> {
        let url = self.base.join("community").expect("url must be valid");
        let response = self
            .client
            .get(url)
            .query(&WithAuth::new(GetCommunity { id }, &self.token))
            .send()
            .await?;

        if response.status().is_success() {
            let community = response.json().await?;
            Ok(community)
        } else {
            let error = response.json::<ServerError>().await?;
            match error.error.as_str() {
                "couldnt_find_community" => Err(CommunityError::NotFound),
                _ => Err(CommunityError::ServerError(error)),
            }
        }
    }

    /// Get / fetch posts, with various filters
    pub async fn get_posts(
        &self,
        type_: ListingType,
        sort: SortType,
        community_id: Option<i32>,
        limit: i32,
    ) -> Result<Vec<PostView>, PostError> {
        let url = self.base.join("post/list").expect("url must be valid");
        let payload = GetPosts {
            type_,
            sort,
            community_id,
            page: 1,
            limit,
        };

        let response = self
            .client
            .get(url)
            .query(&WithAuth::new(payload, &self.token))
            .send()
            .await?;

        if response.status().is_success() {
            let posts_response = response.json::<GetPostsResponse>().await?;
            Ok(posts_response.posts)
        } else {
            let error = response.json().await?;
            Err(PostError::ServerError(error))
        }
    }

    /// List communities, with various filters
    pub async fn list_communities(
        &self,
        type_: ListingType,
        sort: SortType,
        show_nsfw: bool,
        limit: i32,
    ) -> Result<Vec<CommunityView>, CommunityError> {
        let url = self.base.join("community/list").expect("url must be valid");
        let payload = ListCommunities {
            type_,
            sort,
            show_nsfw,
            page: 1,
            limit,
        };

        let response = self
            .client
            .get(url)
            .query(&WithAuth::new(payload, &self.token))
            .send()
            .await?;

        if response.status().is_success() {
            let communities_response = response.json::<ListCommunitiesResponse>().await?;
            Ok(communities_response.communities)
        } else {
            let error = response.json().await?;
            Err(CommunityError::ServerError(error))
        }
    }

    /// Fetch a non-local / federated object
    pub async fn resolve_object(&self, q: &str) -> Result<Option<CommunityView>, ResolveError> {
        let url = self.base.join("resolve_object").expect("url must be valid");

        let response = self
            .client
            .get(url)
            .query(&WithAuth::new(ResolveObject { q }, &self.token))
            .send()
            .await?;

        if response.status().is_success() {
            let resolved = response.json::<ResolveObjectResponse>().await?;
            Ok(resolved.community)
        } else {
            let error = response.json::<ServerError>().await?;
            match error.error.as_str() {
                "invalid_query" => Err(ResolveError::InvalidQuery),
                "couldnt_find_object" => Err(ResolveError::NotFound),
                _ => Err(ResolveError::ServerError(error)),
            }
        }
    }
}

/// Fetch information about the instance
async fn node_info(client: &Client, url: &Url) -> Result<NodeInfoResponse, ConnectError> {
    let url = url.join("/nodeinfo/2.0.json").expect("url must be valid");
    let response = client.get(url).send().await?.error_for_status()?;

    if response.status().is_success() {
        let info = response.json().await?;
        Ok(info)
    } else {
        let error = response.json().await?;
        Err(ConnectError::ServerError(error))
    }
}
