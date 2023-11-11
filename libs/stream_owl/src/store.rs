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

macro_rules! forward_impl {
    ($v:vis $fn_name:ident, $($param:ident: $t:ty),*; $returns:ty) => {
        impl InnerSwitchableStore {
            $v fn $fn_name(&self, $($param: $t),*) -> $returns {
                match self {
                    Self::Disk(inner) => inner.$fn_name($($param),*),
                    Self::Mem(inner) => inner.$fn_name($($param),*),
                }

            }
        }
    };
}

forward_impl!(pub(crate) read_blocking_at, buf: &mut [u8], pos: u64; usize);
forward_impl!(ranges,; RangeSet<u64>);
forward_impl!(pub(crate) size,; Option<u64>);
forward_impl!(pub(crate) gapless_from_till, pos: u64, last_seek: u64; bool);

impl InnerSwitchableStore {
    /// might not write everything, returns n bytes written
    pub(crate) async fn write_at(&self, buf: &[u8], pos: u64) -> usize {
        match self {
            Self::Disk(inner) => inner.write_at(buf, pos).await,
            Self::Mem(inner) => inner.write_at(buf, pos).await,
        }
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
