use std::sync::Arc;

mod config;
mod db;
mod id;
mod reader;
mod subs;

use config::Settings;
use reader::Needed;
use reader::ReadReq;
use reader::Reader;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::task::JoinHandle;
use tracing::error;
use tracing::info;
use tracing::instrument;
use traits::EpisodeId;
use traits::Registration;

pub struct Data {
    reader: Reader,
    config: Arc<Settings>,
    data: Arc<db::Store>,
    leases: Arc<id::Leases>,
    subs: subs::Subs,
    _tempdir: tempfile::TempDir,
}

impl Data {
    pub fn new() -> (Self, JoinHandle<()>) {
        let tempdir = tempfile::tempdir().unwrap();
        let data = db::Store::new(&tempdir).unwrap();
        let data = Arc::new(data);
        let subs = subs::Subs::default();
        let (reader, reader_loop) = Reader::new(data.clone(), subs.clone());

        (
            Data {
                config: Arc::new(Settings {}),
                data,
                leases: id::Leases::new(),
                subs,
                _tempdir: tempdir,
                reader,
            },
            reader_loop,
        )
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
    leases: Arc<id::Leases>,
    reader_tx: mpsc::Sender<ReadReq>,
}

impl traits::DataRStore for DataReader {
    #[instrument(skip_all, ret)]
    fn register(&mut self, tx: Box<dyn traits::DataTx>, description: &'static str) -> Registration {
        self.subs.register(tx, description)
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

    fn sub_episodes(
        &self,
        registration: Registration,
        podcast: traits::PodcastId,
    ) -> Box<dyn traits::DataSub> {
        let sub = self.subs.sub_episodes(registration, podcast);
        match self
            .reader_tx
            .try_send(ReadReq::update_one(registration, Needed::Episodes(podcast)))
        {
            Ok(_) => (),
            Err(TrySendError::Full(_)) => error!("reader pipe full"),
            Err(TrySendError::Closed(_)) => panic!("reader pipe closed"),
        }
        Box::new(sub)
    }

    fn sub_episode_details(
        &self,
        registration: Registration,
        episode: EpisodeId,
    ) -> Box<dyn traits::DataSub> {
        let sub = self.subs.sub_episode_details(registration, episode);
        match self
            .reader_tx
            .try_send(ReadReq::update_one(registration, Needed::EpisodeDetails(episode)))
        {
            Ok(_) => (),
            Err(TrySendError::Full(_)) => error!("reader pipe full"),
            Err(TrySendError::Closed(_)) => panic!("reader pipe closed"),
        }
        Box::new(sub)
    }
}

impl DataWriter {
    #[instrument(skip(self))]
    fn update_all(&mut self, data: Vec<Needed>) {
        match self.reader_tx.try_send(ReadReq::update_all(data)) {
            Ok(_) => (),
            Err(TrySendError::Full(_)) => panic!("reader pipe full, messages are being send to quickly or reader is not processing fast enough"),
            Err(TrySendError::Closed(_)) => panic!("reader shut down before data writers"),
        }
    }
}

impl traits::DataWStore for DataWriter {
    fn podcast_id_gen(&self) -> Box<dyn traits::IdGen> {
        Box::new(id::PodcastIdGen::new(self.data.clone(), self.leases.clone()))
    }
    fn episode_id_gen(&self) -> Box<dyn traits::IdGen> {
        Box::new(id::EpisodeIdGen::new(self.data.clone(), self.leases.clone()))
    }

    #[instrument(skip(self))]
    fn add_podcast(&mut self, podcast: traits::Podcast) {
        self.data.podcasts().insert(&podcast.id, &podcast).unwrap();
        self.update_all(vec![Needed::PodcastList]);
        info!("added podcast")
    }

    fn box_clone(&self) -> Box<dyn traits::DataWStore> {
        Box::new(self.clone())
    }

    fn add_episodes(&mut self, podcast_id: traits::PodcastId, episodes: Vec<traits::Episode>) {
        self.data.episodes().insert(&podcast_id, &episodes).unwrap();
        self.update_all(vec![Needed::Episodes(podcast_id)]);
    }

    fn add_episode_details(&mut self, details: Vec<traits::EpisodeDetails>) {
        use dbstruct::TryExtend;

        let ids = details.iter().map(|e| e.id);
        let pairs = details.iter().map(|e| (&e.id, e));
        self.data.episode_details().try_extend(pairs).unwrap();
        let batch = ids.map(Needed::EpisodeDetails).collect();
        self.update_all(batch);
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
            leases: self.leases.clone(),
            reader_tx: self.reader.read_req_tx(),
        })
    }
}
