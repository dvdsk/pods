use futures_core::stream::Stream;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};

mod config;
mod db;

use config::Settings;
use traits::DataUpdate;

pub struct DataSub;
impl traits::DataSub for DataSub {}

pub struct Data {
    config: Arc<Settings>,
    update_tx: Option<Sender<DataUpdate>>,
    update_rx: Option<Receiver<DataUpdate>>,
}

impl Data {
    pub fn new() -> Self {
        let (update_tx, update_rx) = mpsc::channel(10);
        Data {
            config: Arc::new(Settings {}),
            update_tx: Some(update_tx),
            update_rx: Some(update_rx),
        }
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        Arc::get_mut(&mut self.config).expect("needs to be called before reader or writer")
    }

    pub fn reader(&mut self) -> DataReader {
        assert!(self.update_rx.is_some());
        DataReader {
            config: self.config.clone(),
            update_rx: self.update_rx.take(),
            update_tx: self.update_tx.take().unwrap(),
        }
    }

    pub fn writer(&mut self) -> DataWriter {
        DataWriter
    }
}

pub struct DataReader {
    config: Arc<Settings>,
    update_rx: Option<Receiver<DataUpdate>>,
    update_tx: Sender<DataUpdate>,
}
pub struct DataWriter;

impl traits::DataRStore for DataReader {
    fn updates(&mut self) -> Box<dyn Stream<Item = DataUpdate> + Send> {
        let recv = self.update_rx.take().unwrap();
        let stream = tokio_stream::wrappers::ReceiverStream::new(recv);
        Box::new(stream)
    }

    fn sub_podcasts(&self) -> Box<dyn traits::DataSub> {
        use traits::Podcast;
        let fake_data = vec![Podcast {
            name: "TestPodcast Name".into(),
            id: 0,
        }];
        self.update_tx
            .blocking_send(DataUpdate::Podcasts {
                podcasts: fake_data,
            })
            .unwrap();
        Box::new(DataSub)
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

    fn add_podcast(&mut self, podcast: traits::SearchResult) {
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
    fn updates(&mut self) -> Box<dyn Stream<Item = DataUpdate> + Send> {
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

    fn add_podcast(&mut self, podcast: traits::SearchResult) {
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
