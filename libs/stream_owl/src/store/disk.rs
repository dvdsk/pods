use std::path::Path;

use rangemap::RangeSet;

use super::mem::Memory;
use super::StoreVariant;

#[derive(Debug)]
pub(crate) struct Disk;

impl Disk {
    async fn write_at(&self, buf: &[u8], pos: u64) {
        todo!()
    }

    fn read_blocking_at(&self, buf: &mut [u8], pos: u64) -> usize {
        todo!()
    }
    fn ranges(&self) -> RangeSet<u64> {
        todo!()
    }
    fn variant(&self) -> StoreVariant {
        StoreVariant::Disk
    }
    fn size(&self) -> Option<u64> {
        todo!()
    }
    fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool {
        todo!()
    }
}

impl Disk {
    pub(crate) fn from(memory: &mut Memory, path: &Path) -> Result<Self, ()> {
        todo!()
    }

    pub(crate) fn new(path: &Path) -> Result<Self, ()> {
        todo!()
    }
}
