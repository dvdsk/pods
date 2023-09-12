use std::sync::Arc;

mod config;
mod db;
mod id;
// mod reader;
mod pubsub;

use config::Settings;
use pubsub::Publisher;
use subscriber::PublishTask;
use tokio::sync::mpsc;
use tracing::info;
use tracing::instrument;
use traits::DataUpdateVariant;
use traits::EpisodeId;
use traits::Registration;

pub struct Data {
    publisher: Publisher,
    config: Arc<Settings>,
    data: Arc<db::Store>,
    leases: Arc<id::Leases>,
    _tempdir: tempfile::TempDir,
}

impl Data {
    pub fn new() -> (Self, PublishTask) {
        let tempdir = tempfile::tempdir().unwrap();
        let data = db::Store::new(&tempdir).unwrap();
        let data = Arc::new(data);
        let (publisher, publish_task) = pubsub::new(data.clone());

        (
            Data {
                config: Arc::new(Settings {}),
                data,
                leases: id::Leases::new(),
                _tempdir: tempdir,
                publisher,
            },
            publish_task,
        )
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        Arc::get_mut(&mut self.config).expect("needs to be called before reader or writer")
    }
}

pub struct DataReader {
    config: Arc<Settings>,
    publisher: pubsub::Publisher,
}

#[derive(Clone)]
pub struct DataWriter {
    data: Arc<db::Store>,
    leases: Arc<id::Leases>,
    publisher: pubsub::Publisher,
}

impl traits::DataRStore for DataReader {
    #[instrument(skip_all, ret)]
    fn register(
        &mut self,
        tx: mpsc::Sender<traits::DataUpdate>,
        description: &'static str,
    ) -> Registration {
        self.publisher.register(tx, description)
    }

    #[instrument(skip_all, fields(registration))]
    fn sub_podcasts(&self, registration: Registration) -> Box<dyn traits::DataSub> {
        let sub = self
            .publisher
            .subscribe(registration, DataUpdateVariant::Podcasts);
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
        let sub = self.publisher.subscribe(
            registration,
            DataUpdateVariant::Episodes {
                podcast_id: podcast,
            },
        );
        Box::new(sub)
    }

    fn sub_episode_details(
        &self,
        registration: Registration,
        episode: EpisodeId,
    ) -> Box<dyn traits::DataSub> {
        let sub = self.publisher.subscribe(
            registration,
            DataUpdateVariant::EpisodeDetails {
                episode_id: episode,
            },
        );
        Box::new(sub)
    }
}

impl traits::DataWStore for DataWriter {
    fn podcast_id_gen(&self) -> Box<dyn traits::IdGen> {
        Box::new(id::PodcastIdGen::new(
            self.data.clone(),
            self.leases.clone(),
        ))
    }
    fn episode_id_gen(&self) -> Box<dyn traits::IdGen> {
        Box::new(id::EpisodeIdGen::new(
            self.data.clone(),
            self.leases.clone(),
        ))
    }

    #[instrument(skip(self))]
    fn add_podcast(&mut self, podcast: traits::Podcast) {
        self.data.podcasts().insert(&podcast.id, &podcast).unwrap();
        self.publisher.publish(DataUpdateVariant::Podcasts);
        info!("added podcast")
    }

    fn box_clone(&self) -> Box<dyn traits::DataWStore> {
        Box::new(self.clone())
    }

    fn add_episodes(&mut self, podcast_id: traits::PodcastId, episodes: Vec<traits::Episode>) {
        self.data.episodes().insert(&podcast_id, &episodes).unwrap();
        self.publisher
            .publish(DataUpdateVariant::Episodes { podcast_id });
    }

    fn add_episode_details(&mut self, details: Vec<traits::EpisodeDetails>) {
        use dbstruct::TryExtend;

        let ids = details.iter().map(|e| e.episode_id);
        let pairs = details.iter().map(|e| (&e.episode_id, e));
        self.data.episode_details().try_extend(pairs).unwrap();
        let batch = ids
            .map(|id| DataUpdateVariant::EpisodeDetails { episode_id: id })
            .collect();
        self.publisher.publish_batch(batch);
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
            publisher: self.publisher.clone(),
        })
    }

    fn writer(&mut self) -> Box<dyn traits::DataWStore> {
        Box::new(DataWriter {
            data: self.data.clone(),
            publisher: self.publisher.clone(),
            leases: self.leases.clone(),
        })
    }
}
