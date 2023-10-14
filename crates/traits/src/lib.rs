mod datastore;
mod entities;

use color_eyre::Result;
pub use datastore::*;
pub use entities::*;

use core::fmt;
use std::time::Duration;

pub use async_trait::async_trait;
pub use color_eyre::eyre;
use eyre::WrapErr;
use tokio::sync::{broadcast, mpsc};

pub use std::error::Error;

#[async_trait]
pub trait IntentReciever: Send + fmt::Debug {
    async fn next_intent(&mut self) -> Option<(UserIntent, Box<dyn Updater>)>;
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
    ) -> (Vec<SearchResult>, Result<(), Box<dyn Error + Send>>);
}

pub trait Source {
    fn seek(&mut self, pos: u64);
}

pub trait Media: Send {
    fn get(&mut self, episode_id: EpisodeId, url: url::Url) -> Box<dyn Source>;
    fn download(&mut self, episode_id: EpisodeId, url: url::Url);
    fn cancel_download(&mut self, episode_id: EpisodeId);
}

pub trait Player: Send {
    fn play(&mut self, source: Box<dyn Source>);
    fn pause(&mut self);
    fn stop(&mut self);
    fn seek(&mut self);
}

#[derive(Debug)]
pub struct EpisodeInfo {
    pub stream_url: url::Url,
    pub duration: Duration,
    pub description: String,
    pub title: String,
    pub date: Date,
}

#[async_trait]
pub trait Feed: Send + Sync {
    async fn index(&self, podcast: &Podcast) -> Result<Vec<EpisodeInfo>, Box<dyn Error>>;
    fn box_clone(&self) -> Box<dyn Feed>;
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
    async fn next_intent(&mut self) -> Option<(UserIntent, Box<dyn Updater>)> {
        let intent = self.rx.recv().await?;
        let updater = Box::new(self.tx.clone()) as Box<dyn Updater>;
        Some((intent, updater))
    }
}
