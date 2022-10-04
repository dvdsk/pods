use core::fmt;

pub use async_trait::async_trait;
pub use color_eyre::eyre;
use eyre::WrapErr;
use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone)]
pub enum UserIntent {
    Exit,
    ConnectToRemote,
    DisconnectRemote,
    RefuseRemoteClients,
    FullSearch(String),
}

#[derive(Debug, Clone)]
pub enum AppUpdate {
    Exit,
}

pub struct Forcable<T: Sized> {
    forced: bool,
    value: T,
}

impl<T: Sized> Forcable<T> {
    pub fn new(value: T) -> Self {
        Self {
            forced: false,
            value,
        }
    }
    pub fn new_forced(value: T) -> Self {
        Self {
            forced: true,
            value,
        }
    }
    pub fn get_value(self) -> T {
        self.value
    }
    pub fn is_forced(&self) -> bool {
        self.forced
    }
}

pub trait State: Sized {
    type Config: Config;
    fn config_mut(&mut self) -> &mut Self::Config;
    fn config(&self) -> &Self::Config;
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

pub trait Config: Sized {
    fn remote(&self) -> Forcable<Option<Remote>>;
    fn server(&self) -> Forcable<Option<Server>>;
    fn force_remote(&mut self, val: Option<Remote>);
    fn force_server(&mut self, val: Option<Server>);
}

#[async_trait]
pub trait IntentReciever: Send + fmt::Debug {
    async fn next_intent(&mut self) -> Option<UserIntent>;
}

#[async_trait]
impl IntentReciever for mpsc::Receiver<UserIntent> {
    async fn next_intent(&mut self) -> Option<UserIntent> {
        self.recv().await
    }
}

#[async_trait]
pub trait Updater: Send + fmt::Debug {
    async fn update(&mut self, msg: AppUpdate) -> Result<(), eyre::Report>;
}

#[async_trait]
impl Updater for mpsc::Sender<AppUpdate> {
    async fn update(&mut self, msg: AppUpdate) -> Result<(), eyre::Report> {
        self.send(msg).await.wrap_err("Could not send update")
    }
}

#[async_trait]
impl Updater for broadcast::Sender<AppUpdate> {
    async fn update(&mut self, msg: AppUpdate) -> Result<(), eyre::Report> {
        self.send(msg)
            .map(|_| ())
            .wrap_err("Could not broadcast update")
    }
}

pub trait LocalUI: Send {
    fn ports(&mut self) -> (&mut dyn Updater, &mut dyn IntentReciever);
}

#[async_trait]
pub trait RemoteController: Send + fmt::Debug {
    async fn disable(&mut self);
    async fn enable(&mut self, config: Remote);
}

pub trait RemoteUI: Send + fmt::Debug {
    fn ports(
        &mut self,
    ) -> (
        &mut dyn Updater,
        &mut dyn IntentReciever,
        &mut dyn RemoteController,
    );
    fn controller(&mut self) -> &mut dyn RemoteController;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
}

// /// Joined result type for returning 
// #[must_use]
// pub struct MixedResult<T, E: fmt::Debug> {
//     ok: Vec<T>,
//     err: Vec<E>,
// }
//
// impl<T, E: fmt::Debug> MixedResult<T, E> {
//     pub fn unwrap(self) -> Vec<T> {
//         if !self.err.is_empty() {
//             panic!("{:?}", self.err)
//         }
//
//         self.ok
//     }
// }

#[async_trait]
pub trait IndexSearcher {
    #[must_use]
    async fn search(&mut self, term: &str) -> (Vec<SearchResult>, Result<(), Box<dyn std::error::Error>>);
}
