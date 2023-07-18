mod datastore;
mod entities;

pub use datastore::*;
pub use entities::*;

use core::fmt;

pub use async_trait::async_trait;
pub use color_eyre::eyre;
use eyre::WrapErr;
use tokio::sync::{broadcast, mpsc};

#[async_trait]
pub trait IntentReciever: Send + fmt::Debug {
    async fn next_intent(&mut self) -> Option<(UserIntent, &mut dyn Updater)>;
    // async fn next_intent(&mut self) -> Option<UserIntent>;
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

#[async_trait]
pub trait IndexSearcher: Send {
    #[must_use]
    async fn search(
        &mut self,
        term: &str,
    ) -> (
        Vec<SearchResult>,
        Result<(), Box<dyn std::error::Error + Send>>,
    );
}

#[derive(Debug)]
pub struct LocalIntentReciever {
    rx: mpsc::Receiver<UserIntent>,
    tx: mpsc::Sender<AppUpdate>,
}

impl LocalIntentReciever {
    pub fn new(
        rx: tokio::sync::mpsc::Receiver<UserIntent>,
        tx: tokio::sync::mpsc::Sender<AppUpdate>,
    ) -> Self {
        Self { rx, tx }
    }
}

#[async_trait]
impl IntentReciever for LocalIntentReciever {
    async fn next_intent(&mut self) -> Option<(UserIntent, &mut dyn Updater)> {
        let intent = self.rx.recv().await?;
        let updater = &mut self.tx as &mut dyn Updater;
        Some((intent, updater))
    }
}
