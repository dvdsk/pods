use std::sync::Arc;
mod config;
mod db;
mod reader;
mod subs;

use config::Settings;
use reader::Needed;
use reader::ReadReq;
use reader::Reader;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tracing::error;
use tracing::instrument;
use traits::Registration;

pub struct Data {
    reader: Reader,
    config: Arc<Settings>,
    data: Arc<db::Store>,
    subs: subs::Subs,
    _tempdir: tempfile::TempDir,
}

impl Data {
    pub fn new() -> Self {
        let tempdir = tempfile::tempdir().unwrap();
        let data = db::Store::new(&tempdir).unwrap();
        let data = Arc::new(data);
        let subs = subs::Subs::default();
        let reader = Reader::new(data.clone(), subs.clone());

        Data {
            config: Arc::new(Settings {}),
            data,
            subs,
            _tempdir: tempdir,
            reader,
        }
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        Arc::get_mut(&mut self.config).expect("needs to be called before reader or writer")
    }
}

pub struct DataReader {
    config: Arc<Settings>,
    subs: subs::Subs,
    reader_tx: mpsc::Sender<ReadReq>,
}

#[derive(Clone)]
pub struct DataWriter {
    data: Arc<db::Store>,
    reader_tx: mpsc::Sender<ReadReq>,
}

impl traits::DataRStore for DataReader {
    #[instrument(skip_all, ret)]
    fn register(&mut self, tx: Box<dyn traits::DataTx>) -> Registration {
        self.subs.register(tx)
    }

    #[instrument(skip_all, fields(registration))]
    fn sub_podcasts(&self, registration: Registration) -> Box<dyn traits::DataSub> {
        let sub = self.subs.sub_podcasts(registration);
        match self
            .reader_tx
            .try_send(ReadReq::update_one(registration, Needed::PodcastList))
        {
            Ok(_) => (),
            Err(TrySendError::Full(_)) => error!("reader pipe full"),
            Err(TrySendError::Closed(_)) => panic!("reader pipe closed"),
        }
        Box::new(sub)
    }

    fn settings(&self) -> &dyn traits::Settings {
        self.config.as_ref()
    }
}

impl traits::DataWStore for DataWriter {
    fn add_podcast(&mut self, podcast: traits::Podcast) {
        self.data.podcasts().push(podcast).unwrap();
        match self
            .reader_tx
            .try_send(ReadReq::update_all(Needed::PodcastList))
        {
            Ok(_) => (),
            Err(TrySendError::Full(_)) => error!("reader pipe full"),
            Err(TrySendError::Closed(_)) => panic!("reader pipe closed"),
        }
    }
    fn box_clone(&self) -> Box<dyn traits::DataWStore> {
        Box::new(self.clone())
    }

    fn add_episodes(&mut self, podcast: &traits::Podcast, episodes: Vec<traits::Episode>) {
        todo!()
    }
}

impl traits::LocalOrRemoteStore for DataWriter {
    fn set_remote(&mut self) {
        todo!()
    }

    // we only support local
    fn set_local(&mut self) {}
}

impl traits::LocalOrRemoteStore for Data {
    fn set_remote(&mut self) {
        todo!()
    }

    // we only support local
    fn set_local(&mut self) {}
}

impl traits::DataStore for Data {
    fn reader(&self) -> Box<dyn traits::DataRStore> {
        Box::new(DataReader {
            config: self.config.clone(),
            subs: self.subs.clone(),
            reader_tx: self.reader.read_req_tx(),
        })
    }

    fn writer(&mut self) -> Box<dyn traits::DataWStore> {
        Box::new(DataWriter {
            data: self.data.clone(),
            reader_tx: self.reader.read_req_tx(),
        })
    }
}
