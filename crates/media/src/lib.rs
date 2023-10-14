use traits::{EpisodeId, Source};

use url::Url;

mod streamer;
use streamer::Streamer;
pub use streamer::Handle;

mod local;
use local::Local;

pub struct Media {
    local: Local,
    streamer: Streamer,
}

impl Media {
    #[must_use]
    pub fn new() -> (Self, streamer::Handle) {
        let (streamer, errors) = Streamer::new();
        (
            Media {
                local: Local::new(),
                streamer,
            },
            errors,
        )
    }
}

impl traits::Media for Media {
    fn get(&mut self, episode_id: EpisodeId, url: Url) -> Box<dyn Source> {
        if let Some(local_source) = self.local.get(episode_id) {
            return local_source;
        }

        self.streamer.stream(episode_id, url)
    }
    fn download(&mut self, episode_id: EpisodeId, url: Url) {
        self.streamer.download(episode_id, url)
    }
    fn cancel_download(&mut self, episode_id: EpisodeId) {
        self.streamer.cancel_download(episode_id)
    }
}
