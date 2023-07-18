pub use async_trait::async_trait;
pub use color_eyre::eyre;
use tokio::sync::oneshot;

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
        awnser: oneshot::Sender<Vec<SearchResult>>,
    },
}

#[derive(Debug, Clone)]
pub enum AppUpdate {
    Exit,
}

#[derive(Debug)]
pub enum ReqUpdate {
    Search(oneshot::Receiver<Vec<SearchResult>>),
    CancelSearch,
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