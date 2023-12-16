use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tracing::instrument;

use crate::store::{self, SwitchableStore};

#[derive(Debug, Clone)]
pub(crate) struct StreamTarget {
    /// on seek this pos is updated, in between seeks
    /// it increments with the number of bytes written
    pos: Arc<AtomicU64>,
    store: SwitchableStore,
    pub(crate) chunk_size: u64,
}

impl StreamTarget {
    pub(crate) fn new(store: SwitchableStore, start_pos: u64, chunk_size: usize) -> Self {
        Self {
            store,
            pos: Arc::new(AtomicU64::new(start_pos)),
            chunk_size: chunk_size as u64,
        }
    }

    pub(crate) fn pos(&self) -> u64 {
        self.pos.load(Ordering::Acquire)
    }

    pub(crate) fn set_pos(&self, pos: u64) {
        self.pos.store(pos, Ordering::Release)
    }

    pub(crate) fn next_range(&self, stream_size: Option<u64>) -> std::ops::Range<u64> {
        let start = self.pos();
        let mut end = self.pos() + self.chunk_size;
        if let Some(size) = stream_size {
            end = end.min(size);
        }
        start..end
    }
}

impl StreamTarget {
    #[instrument(level = "trace", skip(self, buf), fields(buf_len = buf.len()))]
    pub(crate) async fn append(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        // only this function modifies pos,
        // only need to read threads own writes => relaxed ordering
        let written = self
            .store
            .write_at(buf, self.pos.load(Ordering::Relaxed))
            .await;

        let bytes = match written {
            Ok(bytes) => bytes.get(),
            Err(store::Error::SeekInProgress) => {
                //
                futures::pending!();
                unreachable!()
            }
            Err(other) => {
                todo!("handle other error: {other:?}")
            }
        };

        // new data needs to be requested after current pos, it uses acquire Ordering
        self.pos.fetch_add(bytes as u64, Ordering::Release);
        Ok(bytes)
    }
}
