use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Certificate, Client, Response,
};
use serde::Serialize;
use std::sync::Arc;
use std::{env, fs};
use url::Url;

mod errors;
mod http;
mod types;

pub use errors::{ConnectError, FetchError, LoginError};
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
#[derive(Clone)]
pub struct LemmyApi {
    client: Client,
    config: Arc<ApiConfig>,
}

struct ApiConfig {
    base: Url,
    token: Option<String>,
}

impl LemmyApi {
    /// Connect to a Lemmy instance
    pub async fn connect(base: &Url) -> Result<LemmyApi, ConnectError> {
        let client = new_client()?;

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
            config: Arc::new(ApiConfig { base, token: None }),
        })
    }

    /// Get the instance name
    pub fn instance(&self) -> &str {
        self.config
            .base
            .host_str()
            .expect("api client must have a host")
    }

    /// Authenticate with the instance
    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), LoginError> {
        let response = self
            .post("user/login", Login { username, password })
            .await?;

        if response.status().is_success() {
            let auth = response.json::<LoginResponse>().await?;
            let token = auth.jwt.ok_or(LoginError::IncorrectCredentials)?;

            let config = Arc::get_mut(&mut self.config).expect("login must occur before cloning");
            config.token = Some(token);

            Ok(())
        } else {
            let error = response.json::<ServerError>().await?;

            match error.error.as_str() {
                "incorrect_login" => Err(LoginError::IncorrectCredentials),
                "email_not_verified" => Err(LoginError::EmailNotVerified),
                _ => Err(LoginError::ServerError(error)),
            }
        }
    }

    /// Follow / subscribe to a community
    pub async fn follow_community(&self, id: i32) -> Result<(), FetchError> {
        let payload = FollowCommunity {
            community_id: id,
            follow: true,
        };

        let response = self.post("community/follow", payload).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error = response.json().await?;
            Err(FetchError::ServerError(error))
        }
    }

    /// Get / fetch a community
    pub async fn get_community(&self, name: &str) -> Result<Option<CommunityResponse>, FetchError> {
        let response = self.get("community", GetCommunity { name }).await?;

        if response.status().is_success() {
            let community = response.json().await?;
            Ok(Some(community))
        } else {
            let error = response.json::<ServerError>().await?;
            match error.error.as_str() {
                "couldnt_find_community" => Ok(None),
                _ => Err(FetchError::ServerError(error)),
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
    ) -> Result<Vec<PostView>, FetchError> {
        let payload = GetPosts {
            type_,
            sort,
            community_id,
            page: 1,
            limit,
        };
        let response = self.get("post/list", payload).await?;

        if response.status().is_success() {
            let posts_response = response.json::<GetPostsResponse>().await?;
            Ok(posts_response.posts)
        } else {
            let error = response.json().await?;
            Err(FetchError::ServerError(error))
        }
    }

    /// List communities, with various filters
    pub async fn list_communities(
        &self,
        type_: ListingType,
        sort: SortType,
        show_nsfw: bool,
        limit: i32,
    ) -> Result<Vec<CommunityView>, FetchError> {
        let payload = ListCommunities {
            type_,
            sort,
            show_nsfw,
            page: 1,
            limit,
        };
        let response = self.get("community/list", payload).await?;

        if response.status().is_success() {
            let communities_response = response.json::<ListCommunitiesResponse>().await?;
            Ok(communities_response.communities)
        } else {
            let error = response.json().await?;
            Err(FetchError::ServerError(error))
        }
    }

    /// Fetch a non-local / federated object
    pub async fn resolve_object(&self, q: &str) -> Result<Option<CommunityView>, FetchError> {
        let response = self.get("resolve_object", ResolveObject { q }).await?;

        if response.status().is_success() {
            let resolved = response.json::<ResolveObjectResponse>().await?;
            Ok(resolved.community)
        } else {
            let error = response.json::<ServerError>().await?;
            match error.error.as_str() {
                "couldnt_find_object" => Ok(None),
                _ => Err(FetchError::ServerError(error)),
            }
        }
    }

    /// Construct a URL from the base
    fn url(&self, path: &str) -> Url {
        self.config.base.join(path).expect("url must be valid")
    }

    /// Send a GET request
    async fn get<T: Serialize>(&self, path: &str, query: T) -> Result<Response, reqwest::Error> {
        self.client
            .get(self.url(path))
            .query(&WithAuth {
                payload: query,
                auth: self.config.token.as_deref(),
            })
            .send()
            .await
    }

    /// Send a POST request
    async fn post<T: Serialize>(&self, path: &str, payload: T) -> Result<Response, reqwest::Error> {
        self.client
            .post(self.url(path))
            .json(&WithAuth {
                payload,
                auth: self.config.token.as_deref(),
            })
            .send()
            .await
    }
}

/// Construct a new HTTP client
fn new_client() -> Result<Client, ConnectError> {
    let mut headers = HeaderMap::new();
    headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

    let mut builder = Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers);

    if let Ok(paths) = env::var("EXTRA_CERTIFICATE_PATHS") {
        for path in paths.split(',') {
            let contents = fs::read(path)?;
            let certificate =
                Certificate::from_pem(&contents).map_err(ConnectError::InvalidCertificate)?;
            builder = builder.add_root_certificate(certificate);
        }
    }

    Ok(builder.build()?)
}

/// Fetch information about the instance
async fn node_info(client: &Client, url: &Url) -> Result<NodeInfoResponse, ConnectError> {
    let url = url.join("/nodeinfo/2.0.json").expect("url must be valid");
    let response = client.get(url).send().await?.error_for_status()?;

    let body = response.text().await?;
    let info = serde_json::from_str(&body).map_err(|_| ConnectError::NotLemmyInstance)?;

    Ok(info)
}
