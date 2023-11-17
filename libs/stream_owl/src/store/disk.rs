use std::path::Path;
use std::sync::Arc;

use rangemap::RangeSet;

use super::capacity::Capacity;
use super::watch;

#[derive(Debug)]
pub(crate) struct Disk {
    pub(super) capacity: Arc<Capacity>,
}

impl Disk {
    pub(super) fn new(
        _path: &Path,
        capacity: Arc<Capacity>,
        _range_tx: watch::Sender,
    ) -> Result<Self, ()> {
        Self {
            capacity,
        };
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
    pub(super) fn size(&self) -> Option<u64> {
        todo!()
    }
    pub(super) fn gapless_from_till(&self, _pos: u64, _last_seek: u64) -> bool {
        todo!()
    }

    pub(super) fn into_range_tx(self) -> watch::Sender {
        todo!()
    }

    pub(super) fn set_range_tx(&mut self, tx: watch::Sender) {
        todo!()
    }

    pub(super) fn last_read_pos(&self) -> u64 {
        todo!()
    }
    pub(super) fn n_supported_ranges(&self) -> usize {
        todo!()
    }
}
