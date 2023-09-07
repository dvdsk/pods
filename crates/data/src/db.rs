use tracing::debug;
use traits::{DataUpdate, Episode, EpisodeDetails, EpisodeId, Podcast, PodcastId};

#[dbstruct::dbstruct(db=sled)]
pub struct Store {
    #[dbstruct(Default)]
    pub podcast_id: u64,
    #[dbstruct(Default)]
    pub episode_id: u64,

    pub downloads: HashMap<EpisodeId, ()>,
    pub podcasts: HashMap<PodcastId, Podcast>,
    pub episodes: HashMap<PodcastId, Vec<Episode>>,
    pub episode_details: HashMap<EpisodeId, EpisodeDetails>,
}

impl Store {
    pub(crate) fn podcast_update(&self) -> DataUpdate {
        let list = self
            .podcasts()
            .values()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        DataUpdate::Podcasts { podcasts: list }
    }

    pub(crate) fn episodes_update(&self, podcast_id: PodcastId) -> DataUpdate {
        let list = match self.episodes().get(&podcast_id).unwrap() {
            None => {
                debug!("No episodes for podcast with id: {podcast_id}");
                Vec::new()
            }
            Some(list) => list,
        };
        DataUpdate::Episodes { podcast_id, list }
    }

    pub(crate) fn episode_details_update(&self, episode_id: EpisodeId) -> DataUpdate {
        let details = match self.episode_details().get(&episode_id).unwrap() {
            None => {
                debug!("No details for episode with id: {episode_id}");
                return DataUpdate::Missing {
                    variant: traits::DataUpdateVariant::EpisodeDetails { episode_id },
                };
            }
            Some(details) => details,
        };
        DataUpdate::EpisodeDetails { details }
    }

    pub(crate) fn downloads_update(&self) -> DataUpdate {
        // let downloads: Result<_, ()> = self.downloads().values().collect();
        DataUpdate::Downloads { list: () }
    }
}
