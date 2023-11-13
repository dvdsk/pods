use rangemap::RangeSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

mod disk;
mod mem;
mod migrate;
mod watch;

pub use migrate::{MigrationError, MigrationHandle};

#[derive(Debug, Clone)]
pub(crate) struct SwitchableStore {
    curr_store: Arc<Mutex<Store>>,
    curr_range: watch::Receiver,
    spare_capacity: Arc<Notify>,
    migrating_to: Option<StoreVariant>,
}

#[derive(Debug)]
pub(crate) enum Store {
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
        let (tx, rx) = watch::channel();
        Self {
            migrating_to: None,
            curr_range: rx,
            spare_capacity: Arc::new(Notify::new()),
            curr_store: Arc::new(Mutex::new(Store::Disk(disk))),
        }
    }

    pub(crate) fn new_mem_backed() -> Self {
        let mem = mem::Memory::new().unwrap();
        let (tx, rx) = watch::channel();
        Self {
            migrating_to: None,
            curr_range: rx,
            spare_capacity: Arc::new(Notify::new()),
            curr_store: Arc::new(Mutex::new(Store::Mem(mem))),
        }
    }

    pub(crate) async fn variant(&self) -> StoreVariant {
        match *self.curr_store.lock().await {
            Store::Disk(_) => StoreVariant::Disk,
            Store::Mem(_) => StoreVariant::Mem,
        }
    }

    pub(super) fn read_blocking_at(&self, buf: &mut [u8], pos: u64) -> usize {
        self.curr_range.blocking_wait_for(pos + 4096); // read at least 4k
        self.curr_store.blocking_lock().read_blocking_at(buf, pos)
    }

    pub(crate) async fn write_at(&mut self, buf: &[u8], pos: u64) -> usize {
        match &mut *self.curr_store.lock().await {
            Store::Disk(inner) => inner.write_at(buf, pos).await,
            Store::Mem(inner) => inner.write_at(buf, pos).await,
        }
    }

    /// refers to the size of the stream if it was complete
    pub(crate) fn size(&self) -> Option<u64> {
        todo!()
    }

    pub(crate) fn gapless_from_till(&self, last_seek: u64, pos: u64) -> bool {
        todo!()
    }
}

macro_rules! forward_impl {
    ($v:vis $fn_name:ident, $($param:ident: $t:ty),*; $returns:ty) => {
        impl Store {
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
// forward_impl!(pub(crate) size,; Option<u64>);
// forward_impl!(pub(crate) gapless_from_till, pos: u64, last_seek: u64; bool);

impl Store {
    /// might not write everything, returns n bytes written
    pub(crate) async fn write_at(&mut self, buf: &[u8], pos: u64) -> usize {
        match self {
            Self::Disk(inner) => inner.write_at(buf, pos).await,
            Self::Mem(inner) => inner.write_at(buf, pos).await,
        }
    }
}
