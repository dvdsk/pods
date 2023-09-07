use std::collections::HashMap;
use traits::{EpisodeId, Source, MediaStore};

use tokio::sync::mpsc;
use super::Stream;
use super::memory::ToMem;

mod status;

/// start a greedy remembering stream to disk.
/// prioritizes data in front of the current position
/// will download everything eventually
pub(crate) struct ToDisk;

impl ToDisk {
    pub fn as_source(&self) -> Box<dyn Source> {
        todo!()
    }

    pub(crate) fn new(tx: &mut mpsc::Sender<()>) -> Self {
        todo!()
    }

    pub(crate) fn to_mem(self) -> ToMem {
        todo!()
    }
}

pub(super) fn load_streams() -> HashMap<EpisodeId, Stream> {
    status::load().iter().map(|_| todo!()).collect()
}

pub(crate) async fn stream_manager(rx: mpsc::Receiver<()>) -> () {
    todo!()
}
