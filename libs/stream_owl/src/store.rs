use std::path::PathBuf;
use std::sync::Arc;

use rangemap::RangeSet;
use tokio::sync::{Mutex, MutexGuard};

mod ringbuffer;

mod disk;
mod mem;
mod migrate;

pub use migrate::{MigrationError, MigrationHandle};

#[derive(Debug, Clone)]
pub(crate) struct SwitchableStore {
    curr: Arc<Mutex<InnerSwitchableStore>>,
    migrating_to: Option<StoreVariant>,
}

#[derive(Debug)]
pub(crate) enum InnerSwitchableStore {
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
}

impl InnerSwitchableStore {
    /// might not write everything, returns n bytes written
    pub(crate) async fn write_at(&self, buf: &[u8], pos: u64) -> usize {
        todo!()
    }
    pub(crate) fn read_blocking_at(&self, buf: &mut [u8], pos: u64) -> usize {
        todo!()
    }
    fn ranges(&self) -> RangeSet<u64> {
        todo!()
    }
    pub(crate) fn size(&self) -> Option<u64> {
        todo!()
    }
    pub(crate) fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool {
        todo!()
    }
}

impl SwitchableStore {
    pub fn blocking_lock(&self) -> MutexGuard<'_, InnerSwitchableStore> {
        self.curr.blocking_lock()
    }
    pub async fn lock(&self) -> MutexGuard<'_, InnerSwitchableStore> {
        self.curr.lock().await
    }
}
