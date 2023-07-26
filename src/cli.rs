use crate::api::SortType;
use clap::Parser;
use std::time::Duration;
use tracing::Level;
use url::Url;

mod parsers;

/// Parse the command line arguments
pub fn parse() -> Args {
    Args::parse()
}

/// Populate your Lemmy instance's All feed with communities and posts
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Args {
    /// The URL of the instance's API
    ///
    /// This can be the URL of the API container or the public URL with the `/api` prefix
    #[arg(long, default_value = "http://127.0.0.1:8536", env = "API_URL")]
    pub url: Url,

    /// The user to act as on your instance
    #[arg(
        long,
        env = "LOCAL_USERNAME",
        value_parser = parsers::string(),
    )]
    pub username: String,
    /// The user's password
    #[arg(
        long,
        env = "LOCAL_PASSWORD",
        value_parser = parsers::string(),
    )]
    pub password: String,

    /// Comma-separated list of domains for the peer servers to pull from
    #[arg(
        long,
        env = "PEERS",
        value_delimiter = ',',
        value_parser = parsers::domain(),
    )]
    pub peers: Vec<String>,
    /// Comma-separated list of domains to ignore posts from
    #[arg(
        long,
        env = "IGNORED",
        value_delimiter = ',',
        value_parser = parsers::domain(), env = "IGNORED",
    )]
    pub ignored: Vec<String>,

    /// The number of posts to pull from each community
    #[arg(long, default_value_t = 50, env = "POST_COUNT")]
    pub post_count: i32,
    /// The number of communities to pull from each instance
    #[arg(long, default_value_t = 25, env = "COMMUNITY_COUNT")]
    pub community_count: i32,

    /// A comma-separated list of the methods to sort communities by to find posts
    #[arg(
        long,
        default_value = "top-all,top-day",
        env = "SORT_METHODS",
        value_delimiter = ',',
        value_enum
    )]
    pub sort_methods: Vec<SortType>,

    /// How long to wait after subscribing to a community
    ///
    /// Supports hours, minutes, and seconds unit specifiers with `h`, `m`, and `s` respectively.
    /// Multiple units can be combined together (i.e `1h30m`). If no units are specified, seconds
    /// are assumed.
    #[arg(
        long,
        default_value = "15s",
        env = "COMMUNITY_ADD_DELAY",
        value_parser = parsers::duration(),
    )]
    pub community_add_delay: Duration,
    /// How long to wait between runs
    ///
    /// Supports hours, minutes, and seconds unit specifiers with `h`, `m`, and `s` respectively.
    /// Multiple units can be combined together (i.e `1h30m`). If no units are specified, seconds
    /// are assumed.
    #[arg(
        long,
        default_value = "6h",
        env = "RUN_INTERVAL",
        value_parser = parsers::duration(),
    )]
    pub run_interval: Duration,

    /// The default level to emit logs at
    ///
    /// Can be overriden for individual components with `--log-targets` or the `LOG_TARGETS`
    /// environment variable.
    #[arg(
        long,
        default_value_t = Level::INFO,
        env = "LOG_LEVEL",
    )]
    pub log_level: Level,

    /// Override the default log level for individual components
    ///
    /// Filter input format: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives
    #[arg(long, env = "LOG_TARGETS")]
    pub log_targets: Option<String>,
}
