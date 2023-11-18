use std::path::Path;

use rangemap::RangeSet;

use super::capacity::Capacity;
use super::range_watch;

#[derive(Debug)]
pub(crate) struct Disk {
    pub(super) capacity: Capacity,
}

impl Disk {
    pub(super) fn new(
        _path: &Path,
        capacity: Capacity,
        _range_tx: range_watch::Sender,
    ) -> Result<Self, ()> {
        Self { capacity };
        todo!()
    }
    pub(super) async fn write_at(&self, _buf: &[u8], _pos: u64) -> usize {
        todo!()
    }

    pub(super) fn read_blocking_at(&self, _buf: &mut [u8], _pos: u64) -> usize {
        todo!()
    }
    pub(super) fn ranges(&self) -> RangeSet<u64> {
        todo!()
    }
    pub(super) fn gapless_from_till(&self, _pos: u64, _last_seek: u64) -> bool {
        todo!()
    }

    pub(super) fn set_range_tx(&mut self, _tx: range_watch::Sender) {
        todo!()
    }

    pub(super) fn last_read_pos(&self) -> u64 {
        todo!()
    }

    pub(super) fn n_supported_ranges(&self) -> usize {
        todo!()
    }

    pub(super) fn set_capacity(&mut self, capacity: Capacity) {
        self.capacity = capacity;
    }

    pub(super) fn into_parts(self) -> (range_watch::Sender, Capacity) {
        todo!()
    }
}
