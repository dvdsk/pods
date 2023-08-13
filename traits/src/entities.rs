pub use async_trait::async_trait;
pub use color_eyre::eyre;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use url::Url;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
}

#[derive(Debug)]
pub enum UserIntent {
    Exit,
    ConnectToRemote,
    DisconnectRemote,
    RefuseRemoteClients,
    FullSearch { query: String },
    AddPodcast(SearchResult),
}

pub type ReturnTx = oneshot::Sender<AppUpdate>;
pub type ReturnRx = oneshot::Receiver<AppUpdate>;

#[derive(Debug, Clone)]
pub enum AppUpdate {
    Exit,
    Error(String),
    SearchResults(Vec<SearchResult>),
}

#[derive(Debug)]
pub enum ReqUpdate {
    Search(oneshot::Receiver<AppUpdate>),
}

/// settings with which to connect to panda server
#[derive(Debug)]
pub struct Remote {
    pub id: u64,
    pub password: Option<String>,
}

/// options for panda server
#[derive(Debug, Clone)]
pub struct Server {
    pub port: Option<u16>,
    pub password: Option<String>,
}

pub type PodcastId = usize;
pub type EpisodeId = usize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Podcast {
    pub name: String,
    pub feed: Url,
    pub id: PodcastId,
}

#[derive(Debug)]
pub struct InvalidPodcastFeedUrl(url::ParseError);
impl Podcast {
    pub fn try_from_searchres(
        res: SearchResult,
        id: PodcastId,
    ) -> Result<Self, InvalidPodcastFeedUrl> {
        let feed = Url::parse(&res.url).map_err(InvalidPodcastFeedUrl)?;
        Ok(Self {
            name: res.title,
            feed,
            id,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub name: String,
    pub id: EpisodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeDetails {
    pub description: String,
    pub id: EpisodeId,
}
