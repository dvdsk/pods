use rangemap::RangeSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

mod capacity;
mod disk;
mod mem;
mod migrate;
mod watch;

pub use migrate::{MigrationError, MigrationHandle};

use self::capacity::Capacity;

#[derive(Debug, Clone)]
pub(crate) struct SwitchableStore {
    curr_store: Arc<Mutex<Store>>,
    curr_range: watch::Receiver,
    capacity: Arc<Capacity>,
    stream_size: Option<u64>,
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
        let capacity = Arc::new(Capacity::new());
        let (tx, rx) = watch::channel();
        let disk = disk::Disk::new(&path, capacity.clone(), tx).unwrap();
        Self {
            curr_range: rx,
            capacity,
            curr_store: Arc::new(Mutex::new(Store::Disk(disk))),
            stream_size: None,
        }
    }

    pub(crate) fn new_mem_backed() -> Self {
        let capacity = Arc::new(Capacity::new());
        let (tx, rx) = watch::channel();
        let mem = mem::Memory::new(capacity.clone(), tx).unwrap();
        Self {
            curr_range: rx,
            capacity,
            curr_store: Arc::new(Mutex::new(Store::Mem(mem))),
            stream_size: None,
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
        self.capacity.wait_for_space().await;
        match &mut *self.curr_store.lock().await {
            Store::Disk(inner) => inner.write_at(buf, pos).await,
            Store::Mem(inner) => inner.write_at(buf, pos).await,
        }
    }

    /// refers to the size of the stream if it was complete
    pub(crate) fn size(&self) -> Option<u64> {
        self.stream_size
    }

    pub(crate) fn gapless_from_till(&self, last_seek: u64, pos: u64) -> bool {
        self.curr_store.blocking_lock().gapless_from_till(pos, last_seek)
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
forward_impl!(pub(crate) gapless_from_till, pos: u64, last_seek: u64; bool);
forward_impl!(ranges,; RangeSet<u64>);
forward_impl!(last_read_pos,; u64);
forward_impl!(n_supported_ranges,; usize);

impl Store {
    /// might not write everything, returns n bytes written
    pub(crate) async fn write_at(&mut self, buf: &[u8], pos: u64) -> usize {
        match self {
            Self::Disk(inner) => inner.write_at(buf, pos).await,
            Self::Mem(inner) => inner.write_at(buf, pos).await,
        }
    }

    fn into_range_tx(self) -> watch::Sender {
        match self {
            Self::Disk(inner) => inner.into_range_tx(),
            Self::Mem(inner) => inner.into_range_tx(),
        }
    }

    fn set_range_tx(&mut self, tx: watch::Sender) {
        match self {
            Self::Disk(inner) => inner.set_range_tx(tx),
            Self::Mem(inner) => inner.set_range_tx(tx),
        }
    }

    fn capacity_handle(&self) -> Arc<Capacity> {
        match self {
            Self::Disk(inner) => inner.capacity.clone(),
            Self::Mem(inner) => inner.capacity.clone(),
        }

    }
}
