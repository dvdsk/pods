use super::disk::Disk;
use super::StoreVariant;
use rangemap::set::RangeSet;

#[derive(Debug)]
pub(crate) struct Memory;

impl Memory {
    pub(crate) fn from(_disk: &mut Disk, _pos: u64) -> Result<Self, ()> {
        todo!()
    }

    pub(crate) fn new() -> Result<Self, ()> {
        todo!()
    }
}

impl Memory {
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
        StoreVariant::Mem
    }
    pub(super) fn size(&self) -> Option<u64> {
        todo!()
    }
    pub(super) fn gapless_from_till(&self, _pos: u64, _last_seek: u64) -> bool {
        todo!()
    }
}
