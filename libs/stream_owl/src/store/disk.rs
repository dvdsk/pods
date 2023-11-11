use std::path::Path;

use rangemap::RangeSet;

use super::mem::Memory;
use super::StoreVariant;

#[derive(Debug)]
pub(crate) struct Disk;

impl Disk {
    pub(super) async fn write_at(&self, _buf: &[u8], _pos: u64) -> usize {
        todo!()
    }

    pub(super) fn read_blocking_at(&self, _buf: &mut [u8], _pos: u64) -> usize {
        todo!()
    }
    pub(super) fn ranges(&self) -> RangeSet<u64> {
        todo!()
    }
    fn variant(&self) -> StoreVariant {
        StoreVariant::Disk
    }
    pub(super) fn size(&self) -> Option<u64> {
        todo!()
    }
    pub(super) fn gapless_from_till(&self, _pos: u64, _last_seek: u64) -> bool {
        todo!()
    }
}

impl Disk {
    pub(crate) fn from(_memory: &mut Memory, _path: &Path) -> Result<Self, ()> {
        todo!()
    }

    pub(crate) fn new(_path: &Path) -> Result<Self, ()> {
        todo!()
    }
}
