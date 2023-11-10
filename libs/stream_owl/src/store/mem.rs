use super::disk::Disk;
use super::StoreVariant;
use rangemap::set::RangeSet;

#[derive(Debug)]
pub(crate) struct Memory;

impl Memory {
    pub(crate) fn from(disk: &mut Disk, pos: u64) -> Result<Self, ()> {
        todo!()
    }

    pub(crate) fn new() -> Result<Self, ()> {
        todo!()
    }
}

impl Memory {
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
        StoreVariant::Mem
    }
    fn size(&self) -> Option<u64> {
        todo!()
    }
    fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool {
        todo!()
    }
}
