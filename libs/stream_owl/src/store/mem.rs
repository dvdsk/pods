use super::StoreTomato;
use super::disk::Disk;
use super::switch::StoreVariant;
use rangemap::set::RangeSet;

#[derive(Debug)]
pub(crate) struct Memory;

impl Memory {
    pub(crate) fn from(disk: &mut Disk, pos: u64) -> Result<Self, ()> {
        todo!()
    }

    pub(crate) fn ranges(&self) -> RangeSet<u64> {
        todo!()
    }
}

#[async_trait::async_trait]
impl StoreTomato for Memory {
    async fn write_at(&self, buf: &[u8], pos: u64) {
        todo!()
    }

    async fn read_at(&self, buf: &mut [u8], pos: u64) -> usize {
        todo!()
    }
    fn ranges(&self) -> RangeSet<u64> {
        todo!()
    }
    fn variant(&self) -> StoreVariant {
        StoreVariant::Mem
    }
}
