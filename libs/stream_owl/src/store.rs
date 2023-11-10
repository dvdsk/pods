use std::path::PathBuf;
use std::sync::Arc;

use rangemap::RangeSet;
use tokio::sync::Mutex;

mod ringbuffer;

mod disk;
mod mem;
mod migrate;

pub use migrate::{MigrationHandle, MigrationError};

#[derive(Debug, Clone)]
pub(crate) struct SwitchableStore {
    curr: Arc<Mutex<InnerSwitchableStore>>,
    migrating_to: Option<StoreVariant>,
}

#[derive(Debug)]
enum InnerSwitchableStore {
    Disk(disk::Disk),
    Mem(mem::Memory),
}

#[derive(Debug, Clone)]
pub(super) enum StoreVariant {
    Disk = 0,
    Mem = 1,
}

impl SwitchableStore {
    pub(crate) fn new_disk_backed(path: PathBuf) -> Self {
        let disk = disk::Disk::new(&path).unwrap();
        let inner = InnerSwitchableStore::Disk(disk);
        Self {
            migrating_to: None,
            curr: Arc::new(Mutex::new(inner)),
        }
    }

    pub(crate) fn new_mem_backed() -> Self {
        let mem = mem::Memory::new().unwrap();
        let inner = InnerSwitchableStore::Mem(mem);
        Self {
            migrating_to: None,
            curr: Arc::new(Mutex::new(inner)),
        }
    }

    // // TODO need two phase switch with background task
    // // moving most data first
    // pub(crate) async fn swith_to_mem_backed(&mut self) -> Result<(), ()> {
    //     self.switch.to_mem();
    //     Ok(())
    // }
    //
    // pub(crate) async fn swith_to_disk_backed(&mut self, path: &Path) -> Result<(), ()> {
    //     self.switch.to_disk(path);
    //     Ok(())
    // }
}

impl InnerSwitchableStore {
    /// might not write everything, returns n bytes written
    async fn write_at(&self, buf: &[u8], pos: u64) -> usize {
        todo!()
    }
    fn read_blocking_at(&self, buf: &mut [u8], pos: u64) -> usize {
        todo!()
    }
    fn ranges(&self) -> RangeSet<u64> {
        todo!()
    }
    fn variant(&self) -> StoreVariant {
        todo!()
    }
    fn size(&self) -> Option<u64> {
        todo!()
    }
    fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool {
        todo!()
    }
}

impl SwitchableStore {
    /// might not write everything, returns n bytes written
    pub async fn write_at(&self, buf: &[u8], pos: u64) -> usize {
        todo!()
    }
    pub fn read_blocking_at(&self, buf: &mut [u8], pos: u64) -> usize {
        todo!()
    }
    pub fn ranges(&self) -> RangeSet<u64> {
        todo!()
    }
    pub fn variant(&self) -> StoreVariant {
        todo!()
    }
    pub fn size(&self) -> Option<u64> {
        todo!()
    }
    pub fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool {
        todo!()
    }
}
