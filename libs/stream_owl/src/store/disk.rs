use std::path::Path;

use rangemap::RangeSet;

use super::mem::Memory;
use super::StoreTomato;
use super::switch::StoreVariant;

#[derive(Debug)]
pub(crate) struct Disk;

#[async_trait::async_trait]
impl StoreTomato for Disk {
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
        StoreVariant::Disk
    }
}

impl Disk {
    pub(crate) fn from(memory: &mut Memory, path: &Path) -> Result<Self, ()> {
        todo!()
    }
}
