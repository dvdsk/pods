use futures_core::stream::Stream;

mod config;
pub use config::*;

use crate::Podcast;
use crate::Remote;
use crate::SearchResult;
use crate::Server;

#[derive(Debug)]
pub enum DataUpdateVariant {
    Podcast,
}

#[derive(Debug, Clone)]
pub enum DataUpdate {
    Podcasts { podcasts: Vec<crate::Podcast> },
}

impl std::cmp::PartialEq<DataUpdate> for DataUpdateVariant {
    fn eq(&self, other: &DataUpdate) -> bool {
        use DataUpdate::*;

        match (self, other) {
            (Self::Podcast, Podcasts { .. }) => true,
        }
    }
}

pub enum DataKey {}
pub trait DataSub {}

pub trait Settings {
    fn remote(&self) -> Forcable<Option<Remote>>;
    fn server(&self) -> Forcable<Option<Server>>;
    fn force_remote(&mut self, val: Option<Remote>);
    fn force_server(&mut self, val: Option<Server>);
}

// TODO register an update channel instead of returning it
pub trait DataRStore: Send {
    fn updates(&mut self) -> Box<dyn Stream<Item = DataUpdate> + Send>;
    /// Get updates until the subscription is dropped
    fn sub_podcasts(&self) -> Box<dyn DataSub>;
    fn settings(&self) -> &dyn Settings;
}
pub trait DataWStore: Send {
    fn update_podcasts(&mut self);
    /// Add a new podcast to the database.
    fn add_podcast(&mut self, podcast: SearchResult);
    fn sub_podcasts(&mut self);
}

pub trait LocalOrRemoteStore {
    // This should block until the switch is completed
    fn set_remote(&mut self);
    // this should block until the switch is completed
    fn set_local(&mut self);
}

pub trait DataStore: DataRStore + DataWStore + LocalOrRemoteStore {
    fn cloned(&mut self) -> Self
    where
        Self: Sized;
}
