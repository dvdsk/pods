use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use tracing::instrument;

macro_rules! tracing_record {
    ($range:ident) => {
        tracing::Span::current().record(
            stringify!($range),
            $range
                .as_ref()
                .map(|Range { start, end }| format!("{start}..{end}")),
        );
    };
}

use crate::http_client::Size;
use crate::store::{self, SwitchableStore};

#[derive(Debug)]
pub(crate) struct StreamTarget {
    /// on seek this pos is updated, in between seeks
    /// it increments with the number of bytes written
    pos: AtomicU64,
    store: SwitchableStore,
    pub(crate) chunk_size: u64,
}

impl StreamTarget {
    pub(crate) fn new(store: SwitchableStore, start_pos: u64, chunk_size: usize) -> Self {
        Self {
            store,
            pos: AtomicU64::new(start_pos),
            chunk_size: chunk_size as u64,
        }
    }

    pub(crate) fn pos(&self) -> u64 {
        self.pos.load(Ordering::Acquire)
    }

    pub(crate) fn set_pos(&self, pos: u64) {
        self.pos.store(pos, Ordering::Release)
    }

    #[instrument(
        level = "info",
        skip(self),
        fields(closest_beyond_curr_pos, closest_to_start),
        ret
    )]
    pub(crate) async fn next_range(&self, stream_size: &Size) -> Option<Range<u64>> {
        let Some(stream_end) = stream_size.known() else {
            let start = self.pos();
            let end = self.pos() + self.chunk_size;
            return Some(start..end);
        };

        let ranges = self.store.curr_store.lock().await.ranges();
        let limit_to_chunk_size = |Range { start, end }| {
            let len: u64 = end - start;
            let len = len.min(self.chunk_size);
            Range {
                start,
                end: start + len,
            }
        };

        assert!(
            stream_end >= self.pos(),
            "pos ({}) should not be bigger then stream_end: {stream_end}",
            self.pos()
        );
        let closest_beyond_curr_pos = ranges
            .gaps(&(self.pos()..stream_end))
            .next()
            .map(limit_to_chunk_size);
        tracing_record!(closest_beyond_curr_pos);

        let closest_to_start = ranges
            .gaps(&(0..stream_end))
            .next()
            .map(limit_to_chunk_size);
        tracing_record!(closest_to_start);

        closest_beyond_curr_pos.or(closest_to_start)
    }
}

impl StreamTarget {
    #[instrument(level = "trace", skip(self, buf), fields(buf_len = buf.len()))]
    pub(crate) async fn append(&self, buf: &[u8]) -> Result<usize, std::io::Error> {
        // only this function modifies pos,
        // only need to read threads own writes => relaxed ordering
        let written = self
            .store
            .write_at(buf, self.pos.load(Ordering::Relaxed))
            .await;

        let bytes = match written {
            Ok(bytes) => bytes.get(),
            Err(store::Error::SeekInProgress) => {
                // this future will be canceld very very soon, so just block
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
