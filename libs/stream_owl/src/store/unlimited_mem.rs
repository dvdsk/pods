use derivative::Derivative;
use std::collections::TryReserveError;
use std::num::NonZeroUsize;
use std::ops::Range;
use tracing::{debug, instrument};

use rangemap::set::RangeSet;

use super::capacity::Capacity;
use super::range_watch;

mod range_store;
use range_store::RangeStore;

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Memory {
    /// just kept for migrations, not actually used
    pub(super) capacity: Capacity,
    #[derivative(Debug = "ignore")]
    buffer: RangeStore,
    /// the range currently being added to
    active_range: Range<u64>,
    #[derivative(Debug = "ignore")]
    range_tx: range_watch::Sender,
    last_read_pos: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Not critical
    #[error("Refusing write while in the middle of a seek")]
    SeekInProgress,
    #[error("Could not get enough memory from the OS")]
    CouldNotAllocate(#[from] CouldNotAllocate),
}

// needed in store::Error as SeekInProgress is separated from
// all other errors there
#[derive(thiserror::Error, Debug)]
#[error("Could not get enough memory from the OS")]
pub struct CouldNotAllocate(#[from] TryReserveError);

impl Memory {
    pub(super) fn new(capacity: Capacity, range_tx: range_watch::Sender) -> Self {
        Self {
            last_read_pos: 0,
            capacity,
            buffer: RangeStore::new(),
            active_range: 0..0,
            range_tx,
        }
    }

    #[tracing::instrument(level="trace", skip(buf), fields(buf_len = buf.len()))]
    pub(super) async fn write_at(&mut self, buf: &[u8], pos: u64) -> Result<NonZeroUsize, Error> {
        assert!(!buf.is_empty());
        if pos != self.active_range.end {
            debug!("refusing write: position not at current range end, seek must be in progress");
            return Err(Error::SeekInProgress);
        }

        self.buffer.append_at(pos, buf).map_err(CouldNotAllocate)?;
        let written = buf.len();

        self.active_range.end += written as u64;
        self.range_tx.send(self.active_range.clone());
        return Ok(NonZeroUsize::new(written).expect("should never be passed a zero sized write"));
    }

    /// we must only get here if there is data in the mem store for us
    pub(super) fn read_at(&mut self, buf: &mut [u8], pos: u64) -> usize {
        debug_assert!(
            pos >= self.active_range.start,
            "No data in store at offset: {pos}"
        );

        let n_copied = self.buffer.copy_at(pos, buf);
        self.last_read_pos = pos;
        n_copied
    }
    pub(super) fn ranges(&self) -> RangeSet<u64> {
        self.buffer.ranges()
    }
    pub(super) fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool {
        self.buffer
            .ranges()
            .gaps(&(pos..last_seek))
            .next()
            .is_none()
    }

    pub(super) fn set_range_tx(&mut self, tx: range_watch::Sender) {
        self.range_tx = tx;
    }

    pub(super) fn set_capacity(&mut self, capacity: Capacity) {
        self.capacity = capacity;
    }

    pub(super) fn last_read_pos(&self) -> u64 {
        self.last_read_pos
    }
    pub(super) fn n_supported_ranges(&self) -> usize {
        usize::MAX
    }
    pub(super) fn into_parts(self) -> (range_watch::Sender, Capacity) {
        let Self {
            capacity, range_tx, ..
        } = self;
        (range_tx, capacity)
    }
    #[instrument(level = "debug")]
    pub(super) fn writer_jump(&mut self, to_pos: u64) {
        debug_assert!(!self.active_range.contains(&to_pos));

        self.capacity.reset();
        self.active_range = to_pos..to_pos;
        self.range_tx.send(to_pos..to_pos);
        self.last_read_pos = to_pos;
    }
}
