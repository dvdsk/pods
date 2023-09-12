use std::collections::HashMap;
use traits::{EpisodeId, Source};

use tokio::sync::mpsc;
use tokio::task::JoinSet;

mod memory;
use memory::ToMem;
mod disk;
use disk::ToDisk;

enum Stream {
    /// a lazy forgetfull stream to memory, assumes
    /// seeking is usually close to the current position.
    ToMem(ToMem),
    /// start a greedy remembering stream to disk.
    /// prioritizes data in front of the current position
    /// will download everything eventually
    ToDisk(ToDisk),
}

pub(crate) struct Streamer {
    streams: HashMap<EpisodeId, Stream>,
    mem_stream_tx: mpsc::Sender<()>,
    disk_stream_tx: mpsc::Sender<()>,
}

pub struct Handle {
    tasks: JoinSet<()>,
}

impl Handle {
    pub async fn errors(&mut self) -> Box<dyn std::any::Any + Send + 'static> {
        let task_error = self
            .tasks
            .join_next()
            .await
            .expect("There are always two tasks")
            .expect_err("The two stream manager tasks never end");
        let task_panic = task_error
            .try_into_panic()
            .expect("Stream manager is never canceld");
        task_panic
    }
}

impl Streamer {
    pub fn new() -> (Self, Handle) {
        let mut tasks = JoinSet::new();
        let (mem_stream_tx, rx) = mpsc::channel(32);
        tasks.spawn(memory::stream_manager(rx));
        let (disk_stream_tx, rx) = mpsc::channel(32);
        tasks.spawn(disk::stream_manager(rx));
        (
            Streamer {
                streams: disk::load_streams(),
                mem_stream_tx,
                disk_stream_tx,
            },
            Handle { tasks },
        )
    }

    /// start a lazy forgetfull stream to memory
    pub(crate) fn stream(&mut self, episode_id: EpisodeId) -> Box<dyn Source> {
        match self.streams.get(&episode_id) {
            Some(Stream::ToMem(stream)) => return stream.as_source(),
            Some(Stream::ToDisk(stream)) => return stream.as_source(),
            None => (),
        }

        let stream = ToMem::new(&mut self.mem_stream_tx);
        let source = stream.as_source();
        self.streams.insert(episode_id, Stream::ToMem(stream));
        source
    }

    /// turn an existing stream to memory into a greedy remembering
    /// stream to disk
    pub(crate) fn download(&mut self, episode_id: EpisodeId) {
        match self.streams.remove(&episode_id) {
            Some(s @ Stream::ToDisk(_)) => {
                self.streams.insert(episode_id, s);
            }
            Some(Stream::ToMem(stream)) => {
                let stream = stream.to_disk();
                self.streams.insert(episode_id, Stream::ToDisk(stream));
            }
            None => {
                let stream = ToDisk::new(&mut self.disk_stream_tx);
                self.streams.insert(episode_id, Stream::ToDisk(stream));
            }
        }
    }

    pub(crate) fn cancel_download(&mut self, episode_id: EpisodeId) {
        match self.streams.remove(&episode_id) {
            Some(Stream::ToDisk(s)) if s.is_playing() => {
                // convert to a memory stream as we are still playing
                let s = s.to_mem();
                self.streams.insert(episode_id, Stream::ToMem(s));
            }
            Some(s @ Stream::ToMem(_)) => {
                // do nothing with memory streams
                // (since we are canceling a download)
                self.streams.insert(episode_id, s);
            }
            _ => (),
        }
    }
}
