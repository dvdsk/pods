use std::collections::TryReserveError;
use std::ops::Range;

use rangemap::RangeSet;
use tracing::{debug, instrument};

struct Entry {
    start: u64,
    data: Vec<u8>,
}

impl Entry {
    fn first() -> Entry {
        Self {
            start: 0,
            data: Vec::with_capacity(10_000),
        }
    }

    fn range(&self) -> Range<u64> {
        self.start..self.start + self.data.len() as u64
    }
}

pub(crate) struct RangeStore {
    buffers: Vec<Entry>,
}

impl RangeStore {
    pub fn new() -> Self {
        let mut buffers = Vec::with_capacity(10);
        buffers.push(Entry::first());

        Self { buffers }
    }

    #[instrument(skip(self, bytes), fields(n_bytes = bytes.len() ))]
    pub fn append_at(&mut self, pos: u64, bytes: &[u8]) -> Result<(), TryReserveError> {
        let entry = self.buffers.iter_mut().find(|e| e.range().end == pos);

        if let Some(entry) = entry {
            let needed = bytes.len().saturating_sub(entry.data.capacity());
            entry.data.try_reserve(needed)?;
            entry.data.extend_from_slice(bytes)
        } else {
            debug!("Adding new buffer");
            self.buffers.push(Entry {
                start: pos,
                data: bytes.to_vec(),
            })
        }
        Ok(())
    }

    pub(crate) fn copy_at(&self, pos: u64, buf: &mut [u8]) -> usize {
        let Some(entry) = self.buffers.iter().find(|e| e.range().contains(&pos)) else {
            return 0;
        };

        let offset_in_entry = (pos - entry.start) as usize;
        let available = entry.data[offset_in_entry..].len();
        let to_copy = available.min(buf.len());
        let to_copy = &entry.data[offset_in_entry..offset_in_entry + to_copy];
        buf[0..to_copy.len()].copy_from_slice(to_copy);
        to_copy.len()
    }

    pub(crate) fn ranges(&self) -> RangeSet<u64> {
        self.buffers
            .iter()
            .map(Entry::range)
            .filter(|r| !r.is_empty())
            .collect()
    }
}
