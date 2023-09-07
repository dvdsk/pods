use tokio::sync::mpsc;
use traits::Source;

use super::disk::ToDisk;

/// a lazy forgetfull stream to memory, assumes
/// seeking is usually close to the current position.
pub(crate) struct ToMem;

impl ToMem {
    pub(crate) fn new(tx: &mut mpsc::Sender<()>) -> Self {
        todo!()
    }

    pub fn as_source(&self) -> Box<dyn Source> {
        todo!()
    }

    pub fn to_disk(&self) -> ToDisk {
        todo!()
    }
}

pub(crate) async fn stream_manager(rx: mpsc::Receiver<()>) -> () {
    todo!()
}
