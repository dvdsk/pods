pub use async_trait::async_trait;
pub use color_eyre::eyre;
use tokio::sync::oneshot;

use crate::Updater;

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
    FullSearch {
        query: String,
        awnser: ReturnTx,
    },
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

#[derive(Debug, Clone)]
pub struct Podcast {
    pub name: String,
    pub id: PodcastId,
}

