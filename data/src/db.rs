use tracing::debug;
use traits::{DataUpdate, Episode, EpisodeDetails, Podcast, PodcastId, EpisodeId};

#[dbstruct::dbstruct(db=sled)]
pub struct Store {
    pub podcasts: HashMap<PodcastId, Podcast>,
    pub episodes: HashMap<PodcastId, Vec<Episode>>,
    pub episode_details: HashMap<EpisodeId, EpisodeDetails, >,
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
}
