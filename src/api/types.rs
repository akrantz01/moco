use serde::{Deserialize, Serialize};
use url::Url;

/// Allows retrieving a community and it's status from the view
pub trait CommunityViewable {
    fn community(&self) -> &Community;
    fn subscribed(&self) -> SubscribedType;
    fn blocked(&self) -> bool;
}

/// A listing type for post and comment list fetches
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ListingType {
    /// Content from your own site, as well as connected / federated sites
    All,
    /// Content from your site only
    Local,
    /// Content only from communities you've subscribed to
    Subscribed,
}

/// A community
#[derive(Debug, Deserialize)]
pub struct Community {
    pub id: i32,
    pub name: String,
    /// A longer title, that can contain other characters, and doesn't have to be unique
    pub title: String,
    /// Whether the community is removed by a mod.
    pub removed: bool,
    /// Whether its an NSFW community
    pub nsfw: bool,
    /// The federated actor_id
    pub actor_id: Url,
    /// Whether the community is hidden
    pub hidden: bool,
}

/// A community view
#[derive(Debug, Deserialize)]
pub struct CommunityView {
    pub community: Community,
    pub subscribed: SubscribedType,
    pub blocked: bool,
}

impl CommunityViewable for CommunityView {
    fn community(&self) -> &Community {
        &self.community
    }

    fn subscribed(&self) -> SubscribedType {
        self.subscribed
    }

    fn blocked(&self) -> bool {
        self.blocked
    }
}

/// A post view
#[derive(Debug, Deserialize)]
pub struct PostView {
    pub community: Community,
    pub subscribed: SubscribedType,
    pub creator_blocked: bool,
}

impl CommunityViewable for PostView {
    fn community(&self) -> &Community {
        &self.community
    }

    fn subscribed(&self) -> SubscribedType {
        self.subscribed
    }

    fn blocked(&self) -> bool {
        self.creator_blocked
    }
}

/// A server error
#[derive(Debug, Deserialize)]
pub struct ServerError {
    pub error: String,
    pub message: Option<String>,
}

/// The post sort types
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SortType {
    /// Calculates a rank based on the score and time of the latest comment, with decay over time
    Active,
    /// Like active, but uses time when the post was published
    Hot,
    /// Shows most recent posts first
    New,
    /// Shows oldest posts first
    Old,
    /// Highest scoring posts during the last 24 hours
    TopDay,
    /// Highest scoring posts during the last 7 days
    TopWeek,
    /// Highest scoring posts during the last 30 days
    TopMonth,
    /// Highest scoring posts during the last 12 months
    TopYear,
    /// Highest scoring posts during all time
    TopAll,
    /// Shows posts with highest number of comments first
    MostComments,
    /// Bumps posts to the top when they receive a new reply analogous to the sorting of traditional forums
    NewComments,
    /// Highest scoring posts during the last hour
    TopHour,
    /// Highest scoring posts during the last 6 hours
    TopSixHour,
    /// Highest scoring posts during the last 12 hours
    TopTwelveHour,
    /// Highest scoring posts during the last 3 months
    TopThreeMonths,
    /// Highest scoring posts during the last 6 months
    TopSixMonths,
    /// Highest scoring posts during the last 9 months
    TopNineMonths,
}

/// A type / status for a community subscribe
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SubscribedType {
    Subscribed,
    NotSubscribed,
    Pending,
}
