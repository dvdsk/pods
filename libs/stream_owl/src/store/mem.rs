pub const CAPACITY: Option<std::num::NonZeroU64> = NonZeroU64::new(20_000_000);

use std::collections::{TryReserveError, VecDeque};
use std::num::NonZeroU64;
use std::ops::Range;

use rangemap::set::RangeSet;

use super::capacity::Capacity;
use super::range_watch;

#[derive(Debug)]
pub(crate) struct Memory {
    /// space availible because the reader read past data
    pub(super) capacity: Capacity,
    buffer: VecDeque<u8>,
    buffer_cap: usize,
    /// range of positions availible, end non inclusive
    /// positions are measured from the absolute start of the stream
    range: Range<u64>,
    range_tx: range_watch::Sender,
    last_read_pos: u64,
}

impl Memory {
    pub(super) fn new(
        capacity: Capacity,
        range_tx: range_watch::Sender,
    ) -> Result<Self, TryReserveError> {

        let mut bytes = VecDeque::new();
        let buffer_cap = capacity.total().expect("is Some for mem").get() as usize;
        bytes.try_reserve_exact(buffer_cap)?;

        Ok(Self {
            last_read_pos: 0,
            capacity,
            buffer: bytes,
            buffer_cap,
            range: 0..0,
            range_tx,
        })
    }

    // waits until the reader has advanced far enough providing backpressure
    // to the stream

    /// `pos` must be the position of the first byte in buf in the stream.
    /// For the first write at the start this should be 0
    pub(super) async fn write_at(&mut self, buf: &[u8], pos: u64) -> usize {
        if pos != self.range.end {
            self.buffer.clear();
            self.range.start = pos;
        }

        let to_write = buf.len().min(self.capacity.availible());

        let free_in_buffer = self.buffer_cap - self.buffer.len();
        let to_remove = to_write.saturating_sub(free_in_buffer);
        self.buffer.drain(..to_remove);

        self.buffer.extend(buf[0..to_write].iter());

        self.capacity.remove(to_write);
        self.range.start += to_remove as u64;
        self.range.end += to_write as u64;
        self.range_tx.send(self.range.clone());
        return to_write;
    }

    /// we must only get here if there is data in the mem store for us
    pub(super) fn read_blocking_at(&mut self, buf: &mut [u8], pos: u64) -> usize {
        debug_assert!(self.range.start <= pos);

        let relative_pos = pos - self.range.start;
        let n_copied = fill_with_offset(buf, &self.buffer, relative_pos as usize);
        self.capacity.add(n_copied);

        self.last_read_pos = pos;
        n_copied
    }
    pub(super) fn ranges(&self) -> RangeSet<u64> {
        let mut res = RangeSet::new();
        res.insert(self.range.clone());
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
}

fn fill_with_offset(target: &mut [u8], source: &VecDeque<u8>, start: usize) -> usize {
    let (front, back) = source.as_slices();
    if front.len() >= start {
        let n_to_copy = front.len() - start;
        let n_to_copy = n_to_copy.min(target.len());
        target[..n_to_copy].copy_from_slice(&front[start..start + n_to_copy]);
        let n_copied = n_to_copy;

        // copy remaining needed bytes from back
        let n_to_copy = target.len().saturating_sub(n_copied);
        let n_to_copy = n_to_copy.min(back.len());
        target[n_copied..n_to_copy].copy_from_slice(&back[..n_to_copy]);

        n_copied + n_to_copy
    } else {
        let n_to_copy = back.len() - start;
        let n_to_copy = n_to_copy.min(target.len());
        target[..n_to_copy].copy_from_slice(&back[start..start + n_to_copy]);
        let n_copied = n_to_copy;

        n_copied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_with_offset() {
        todo!()
    }
}
