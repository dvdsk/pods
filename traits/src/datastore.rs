use async_trait::async_trait;

mod config;
pub use config::*;

use crate::EpisodeDetails;
use crate::EpisodeId;
use crate::PodcastId;
use crate::Remote;
use crate::Server;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum DataUpdateVariant {
    Podcasts,
    Episodes { podcast_id: PodcastId },
    EpisodeDetails { episode_id: EpisodeId },
}

#[derive(Debug, Clone)]
pub enum DataUpdate {
    Podcasts {
        podcasts: Vec<crate::Podcast>,
    },
    Episodes {
        podcast_id: PodcastId,
        list: Vec<crate::Episode>,
    },
    Missing {
        variant: DataUpdateVariant,
    },
    Placeholder, // prevents irrifutable pattern warns
    EpisodeDetails {
        details: EpisodeDetails,
    },
}

impl DataUpdate {
    pub fn variant(&self) -> DataUpdateVariant {
        use DataUpdateVariant::*;

        match self {
            Self::Podcasts { .. } => Podcasts,
            Self::Episodes { podcast_id, .. } => Episodes {
                podcast_id: *podcast_id,
            },
            Self::EpisodeDetails { details } => EpisodeDetails {
                episode_id: details.id,
            },
            Self::Placeholder => panic!("placeholder should never be used"),
            Self::Missing { .. } => panic!("can not wait for data no being there"),
        }
    }
}

pub enum DataKey {}
pub trait DataSub: Send {}
#[derive(Debug, Clone, Copy)]
pub struct Registration {
    id: usize,
    description: &'static str,
}

impl Registration {
    pub fn new(id: usize, description: &'static str) -> Self {
        Self { id, description }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn description(&self) -> &'static str {
        self.description
    }
}

#[async_trait]
pub trait DataTx: Send {
    async fn send(&mut self, msg: DataUpdate);
    fn box_clone(&self) -> Box<dyn DataTx>;
}

#[async_trait]
impl DataTx for tokio::sync::mpsc::Sender<DataUpdate> {
    async fn send(&mut self, msg: DataUpdate) {
        // segfault in tokio, make async in the future?
        self.try_send(msg).unwrap();
    }
    fn box_clone(&self) -> Box<dyn DataTx> {
        Box::new(self.clone())
    }
}

pub trait Settings {
    fn remote(&self) -> Forcable<Option<Remote>>;
    fn server(&self) -> Forcable<Option<Server>>;
    fn force_remote(&mut self, val: Option<Remote>);
    fn force_server(&mut self, val: Option<Server>);
}

pub trait DataRStore: Send {
    /// Need to register before subscribing
    fn register(&mut self, tx: Box<dyn DataTx>, description: &'static str) -> Registration;
    /// Get updates until the subscription is dropped
    fn sub_podcasts(&self, registration: Registration) -> Box<dyn DataSub>;
    fn sub_episodes(&self, registration: Registration, podcast: PodcastId) -> Box<dyn DataSub>;
    fn settings(&self) -> &dyn Settings;
}
pub trait DataWStore: Send {
    /// Add a new podcast to the database.
    fn add_podcast(&mut self, podcast: crate::Podcast);
    fn add_episodes(&mut self, podcast_id: crate::PodcastId, episodes: Vec<crate::Episode>);
    fn add_episode_details(&mut self, details: Vec<crate::EpisodeDetails>);
    fn box_clone(&self) -> Box<dyn DataWStore>;
}

pub trait LocalOrRemoteStore {
    // This should block until the switch is completed
    fn set_remote(&mut self);
    // this should block until the switch is completed
    fn set_local(&mut self);
}

pub trait DataStore: LocalOrRemoteStore + Send {
    fn writer(&mut self) -> Box<dyn DataWStore>;
    fn reader(&self) -> Box<dyn DataRStore>;
}
