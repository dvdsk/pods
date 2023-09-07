use traits::{EpisodeId, Source};

pub(crate) struct Local {}

impl Local {
    pub fn new() -> Self {
        Local {}
    }

    pub(crate) fn get(&self, episode_id: EpisodeId) -> Option<Box<dyn Source>> {
        todo!()
    }
}
