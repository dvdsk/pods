mod config;
pub use config::*;

use crate::Remote;
use crate::Server;

#[derive(Debug, Clone)]
pub enum DataUpdate {}

pub enum DataKey {}
pub trait DataSub {}

pub trait Settings {
    fn remote(&self) -> Forcable<Option<Remote>>;
    fn server(&self) -> Forcable<Option<Server>>;
    fn force_remote(&mut self, val: Option<Remote>);
    fn force_server(&mut self, val: Option<Server>);
}

pub trait DataRStore: Send {
    fn get_podcasts(&self) -> Box<dyn DataSub>;
    fn settings(&self) -> &dyn Settings;
}
pub trait DataWStore: Send {
    fn update_podcasts(&mut self);
    fn sub_podcasts(&mut self);
}

pub trait DataRWStore: DataRStore + DataWStore {}

pub trait LocalOrRemoteStore {
    // This should block until the switch is completed
    fn set_remote(&mut self);
    // this should block until the switch is completed
    fn set_local(&mut self);
}

pub trait DataStore: DataRWStore + LocalOrRemoteStore {}
