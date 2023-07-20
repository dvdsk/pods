use futures_core::stream::Stream;
use std::sync::Arc;

mod config;
mod db;

use config::Settings;
use traits::DataUpdate;

pub struct Data {
    config: Arc<Settings>,
}

impl Data {
    pub fn new() -> Self {
        Data {
            config: Arc::new(Settings {}),
        }
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        Arc::get_mut(&mut self.config).expect("needs to be called before reader or writer")
    }

    pub fn reader(&self) -> DataReader {
        DataReader {
            config: self.config.clone(),
        }
    }

    pub fn writer(&self) -> DataWriter {
        DataWriter
    }
}

pub struct DataReader {
    config: Arc<Settings>,
}
pub struct DataWriter;

impl traits::DataRStore for DataReader {
    fn updates(&self) -> Box<dyn Stream<Item = DataUpdate> + Send> {
        Box::new(futures::stream::empty())
    }

    fn sub_podcasts(&self) -> Box<dyn traits::DataSub> {
        todo!()
    }

    fn settings(&self) -> &dyn traits::Settings {
        self.config.as_ref()
    }
}

impl traits::DataWStore for DataWriter {
    fn update_podcasts(&mut self) {
        todo!()
    }

    fn sub_podcasts(&mut self) {
        todo!()
    }
}

impl traits::LocalOrRemoteStore for DataWriter {
    fn set_remote(&mut self) {
        todo!()
    }

    fn set_local(&mut self) {
        todo!()
    }
}

impl traits::DataRStore for Data {
    fn updates(&self) -> Box<dyn Stream<Item = DataUpdate> + Send> {
        todo!()
    }

    fn sub_podcasts(&self) -> Box<dyn traits::DataSub> {
        todo!()
    }

    fn settings(&self) -> &dyn traits::Settings {
        self.config.as_ref()
    }
}

impl traits::DataWStore for Data {
    fn update_podcasts(&mut self) {
        todo!()
    }

    fn sub_podcasts(&mut self) {
        todo!()
    }
}

impl traits::LocalOrRemoteStore for Data {
    fn set_remote(&mut self) {
        todo!()
    }

    fn set_local(&mut self) {
        todo!()
    }
}

impl traits::DataStore for Data {}
