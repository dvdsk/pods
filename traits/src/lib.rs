pub use async_trait::async_trait;
use core::fmt;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum UserIntent {
    Exit,
    ConnectToRemote,
    DisconnectRemote,
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

pub struct Remote {
    pub id: u64,
    pub password: Option<String>,
}

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
pub trait IntentReciever: Send {
    async fn next_intent(&mut self) -> Result<UserIntent, Box<dyn fmt::Display>>;
}

#[async_trait]
impl IntentReciever for mpsc::Receiver<UserIntent> {
    async fn next_intent(&mut self) -> Result<UserIntent, Box<dyn fmt::Display>> {
        self.recv().await.ok_or(Box::new("Channel was closed"))
    }
}

#[async_trait]
pub trait Updater: Send {
    async fn update(&mut self, msg: AppUpdate) -> Result<(), Box<dyn fmt::Display>>;
}

pub trait RemoteUI: Send {
    fn disable(&mut self);
    fn enable(&mut self, config: Remote);
    fn updater(&self) -> Box<dyn Updater>;
    fn intent(&self) -> Box<dyn IntentReciever>;
}
