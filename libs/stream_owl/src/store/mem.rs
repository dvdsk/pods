use std::collections::{TryReserveError, VecDeque};
use std::num::NonZeroU64;
use std::ops::Range;
use std::sync::Arc;

use rangemap::set::RangeSet;

use super::capacity::Capacity;
use super::watch;

#[derive(Debug)]
pub(crate) struct Memory {
    /// space availible because the reader read past data
    pub(super) capacity: Arc<Capacity>,
    buffer: VecDeque<u8>,
    buffer_cap: usize,
    /// range of positions availible, end non inclusive
    /// positions are measured from the absolute start of the stream
    range: Range<u64>,
    range_tx: watch::Sender,
}

impl Memory {
    pub(super) fn new(
        capacity: Arc<Capacity>,
        range_tx: watch::Sender,
    ) -> Result<Self, TryReserveError> {
        let buffer_cap = 20_000_000usize;
        capacity.set_total(NonZeroU64::new(buffer_cap as u64));

        let mut bytes = VecDeque::new();
        bytes.try_reserve_exact(buffer_cap)?;

        Ok(Self {
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
    pub(super) fn read_blocking_at(&self, buf: &mut [u8], pos: u64) -> usize {
        debug_assert!(self.range.start <= pos);

        let relative_pos = pos - self.range.start;
        let n_copied = fill_with_offset(buf, &self.buffer, relative_pos as usize);
        self.capacity.add(n_copied);

        n_copied
    }
    pub(super) fn ranges(&self) -> RangeSet<u64> {
        todo!()
    }
    pub(super) fn gapless_from_till(&self, _pos: u64, _last_seek: u64) -> bool {
        todo!()
    }

    pub(super) fn into_range_tx(self) -> watch::Sender {
        self.range_tx
    }

    pub(super) fn set_range_tx(&mut self, tx: watch::Sender) {
        self.range_tx = tx;
    }

    pub(super) fn last_read_pos(&self) -> u64 {
        todo!()
    }
    pub(super) fn n_supported_ranges(&self) -> usize {
        1
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
