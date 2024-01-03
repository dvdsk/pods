use derivative::Derivative;
use std::collections::{TryReserveError, VecDeque};
use std::num::NonZeroUsize;
use std::ops::Range;
use tracing::{debug, instrument};

use rangemap::set::RangeSet;

use crate::vecdeque::VecDequeExt;

use super::capacity::Capacity;
use super::{range_watch, CapacityBounds};

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Memory {
    /// space available because the reader read past data
    pub(super) capacity: Capacity,
    #[derivative(Debug = "ignore")]
    buffer: VecDeque<u8>,
    buffer_cap: usize,
    /// range of positions available, end non inclusive
    /// positions are measured from the absolute start of the stream
    range: Range<u64>,
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
    CouldNotAllocate(#[from] TryReserveError),
}

impl Memory {
    pub(super) fn new(capacity: Capacity, range_tx: range_watch::Sender) -> Result<Self, Error> {
        let mut buffer = VecDeque::new();
        let buffer_cap = match capacity.total() {
            CapacityBounds::Unlimited => 0,
            CapacityBounds::Limited(bytes) => bytes.get() as usize,
        };

        if let CapacityBounds::Limited(bytes) = capacity.total() {
            buffer.try_reserve_exact(bytes.get() as usize)?;
        }

        Ok(Self {
            last_read_pos: 0,
            capacity,
            buffer,
            buffer_cap,
            range: 0..0,
            range_tx,
        })
    }

    // waits until the reader has advanced far enough providing backpressure
    // to the stream

    /// `pos` must be the position of the first byte in buf in the stream.
    /// For the first write at the start this should be 0
    #[tracing::instrument(level="trace", skip(buf), fields(buf_len = buf.len()))]
    pub(super) async fn write_at(&mut self, buf: &[u8], pos: u64) -> Result<NonZeroUsize, Error> {
        assert!(!buf.is_empty());
        if pos != self.range.end {
            debug!("refusing write: position not at current range end, seek must be in progress");
            return Err(Error::SeekInProgress);
        }

        let to_write = buf.len().min(self.capacity.available());

        let free_in_buffer = self.buffer_cap - self.buffer.len();
        let to_remove = to_write.saturating_sub(free_in_buffer);
        self.buffer.drain(..to_remove);
        let removed = to_remove;

        self.buffer.extend(buf[0..to_write].iter());
        let written = to_write;

        self.capacity.remove(written);
        self.range.start += removed as u64;
        self.range.end += written as u64;
        self.range_tx.send(self.range.clone());
        return Ok(NonZeroUsize::new(to_write).expect("just checked if there is capacity to write"));
    }

    /// we must only get here if there is data in the mem store for us
    pub(super) fn read_at(&mut self, buf: &mut [u8], pos: u64) -> usize {
        debug_assert!(pos >= self.range.start, "No data in store at offset: {pos}");

        let relative_pos = pos - self.range.start;
        let n_copied = self.buffer.copy_starting_at(relative_pos as usize, buf);
        self.capacity.add(n_copied);

        self.last_read_pos = pos;
        n_copied
    }
    pub(super) fn ranges(&self) -> RangeSet<u64> {
        let mut res = RangeSet::new();
        if !self.range.is_empty() {
            res.insert(self.range.clone());
        }
        res
    }
    pub(super) fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool {
        self.range.contains(&pos) && self.range.contains(&last_seek)
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
        1
    }
    pub(super) fn into_parts(self) -> (range_watch::Sender, Capacity) {
        let Self {
            capacity, range_tx, ..
        } = self;
        (range_tx, capacity)
    }
    #[instrument(level = "debug")]
    pub(super) fn writer_jump(&mut self, to_pos: u64) {
        debug_assert!(!self.range.contains(&to_pos));

        self.buffer.clear();
        self.capacity.reset();
        self.range = to_pos..to_pos;
        self.range_tx.send(to_pos..to_pos);
        self.last_read_pos = to_pos;
    }
}
