use std::collections::{TryReserveError, VecDeque};
use std::ops::Range;

use rangemap::set::RangeSet;

#[derive(Debug, Clone)]
pub(crate) struct Memory {
    buffer: VecDeque<u8>,
    range: Range<u64>,
    capacity: usize,
}

#[derive(Debug)]
struct Inner {}

impl Memory {
    pub(crate) fn new() -> Result<Self, TryReserveError> {
        let capacity = 20_000_000;

        let mut bytes = VecDeque::new();
        bytes.try_reserve_exact(capacity)?;

        Ok(Self {
            buffer: bytes,
            range: 0..0,
            capacity,
        })
    }

    pub(crate) async fn variant(&self) -> super::StoreVariant {
        todo!()
    }
}

enum ReadErr {
    BufToLarge,
}

impl Memory {
    // waits until the reader has advanced far enough providing backpressure
    // to the stream
    pub(super) async fn write_at(&mut self, buf: &[u8], pos: u64) -> usize {
        todo!("deal with pos");
        todo!("update range locally and send range_tx update");
        todo!("block if dropping goes past current seek (use read_blocking_at)");

        let to_write = buf.len().min(self.capacity);
        self.buffer.drain(..to_write);
        self.buffer.extend(buf[0..to_write].iter());
        return to_write;
    }

    /// we must only get here if there is data in the mem store for us
    pub(super) fn read_blocking_at(&self, buf: &mut [u8], pos: u64) -> usize {
        debug_assert!(self.range.start <= pos);

        let relative_pos = pos - self.range.start;
        let n_copied = fill_with_offset(buf, &self.buffer, relative_pos as usize);

        n_copied
    }
    pub(super) fn ranges(&self) -> RangeSet<u64> {
        todo!()
    }
    pub(super) fn size(&self) -> Option<u64> {
        todo!()
    }
    pub(super) fn gapless_from_till(&self, _pos: u64, _last_seek: u64) -> bool {
        todo!()
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
    fn test_fill_with_offset() {}
}
