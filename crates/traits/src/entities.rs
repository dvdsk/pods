pub use async_trait::async_trait;
pub use color_eyre::eyre;
use std::time::Duration;

use chrono::{DateTime, Local, Utc};
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
    Play(EpisodeId),
    Download(PodcastId),
    CancelDownload(PodcastId),
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

pub type PodcastId = u64;
pub type EpisodeId = u64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Podcast {
    pub name: String,
    pub feed: Url,
    pub id: PodcastId,
}

#[derive(Debug)]
pub enum InvalidPodcastFeedUrl {
    Parse(url::ParseError),
    WrongScheme { is: String },
}

impl Podcast {
    pub fn try_from_searchres(
        res: SearchResult,
        id: PodcastId,
    ) -> Result<Self, InvalidPodcastFeedUrl> {
        let feed = Url::parse(&res.url).map_err(InvalidPodcastFeedUrl::Parse)?;
        if feed.scheme() != "http" && feed.scheme() != "https" {
            return Err(InvalidPodcastFeedUrl::WrongScheme {
                is: feed.scheme().into(),
            });
        }

        Ok(Self {
            name: res.title,
            feed,
            id,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Episode {
    pub name: String,
    pub id: EpisodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EpisodeDetails {
    pub episode_id: EpisodeId,
    pub date: Date,
    pub duration: Duration,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum Date {
    Publication(DateTime<Utc>),
    Added(DateTime<Utc>),
}

impl Date {
    pub fn inner(&self) -> &DateTime<Utc> {
        match self {
            Self::Publication(d) => d,
            Self::Added(d) => d,
        }
    }
    pub fn format(&self) -> String {
        let local: DateTime<Local> = self.inner().clone().into();
        let since = local.signed_duration_since(Local::now());
        if since.num_days() > 30 {
            return format!("{}", local.format("%d:%m:%Y"));
        }
        if since.num_hours() > 48 {
            return format!("{} days ago", since.num_days());
        }

        format!("{} hours ago", since.num_hours())
    }
}
