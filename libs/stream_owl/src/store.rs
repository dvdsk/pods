use rangemap::RangeSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

mod capacity;
mod disk;
mod mem;
mod migrate;
mod range_watch;

pub use migrate::{MigrationError, MigrationHandle};

use capacity::CapacityWatcher;

use self::capacity::Capacity;

#[derive(Debug, Clone)]
pub(crate) struct SwitchableStore {
    curr_store: Arc<Mutex<Store>>,
    curr_range: range_watch::Receiver,
    capacity_watcher: CapacityWatcher,
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
        let (capacity_watcher, capacity) = capacity::new(None);
        let (tx, rx) = range_watch::channel();
        let disk = disk::Disk::new(&path, capacity, tx).unwrap();
        Self {
            curr_range: rx,
            capacity_watcher,
            curr_store: Arc::new(Mutex::new(Store::Disk(disk))),
            stream_size: None,
        }
    }

    pub(crate) fn new_mem_backed() -> Self {
        let (capacity_watcher, capacity) = capacity::new(mem::CAPACITY);
        let (tx, rx) = range_watch::channel();
        let mem = mem::Memory::new(capacity, tx).unwrap();
        Self {
            curr_range: rx,
            capacity_watcher,
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

    pub(super) fn read_blocking_at(&mut self, buf: &mut [u8], pos: u64) -> usize {
        self.curr_range.blocking_wait_for(pos + 4096); // read at least 4k
        self.curr_store.blocking_lock().read_blocking_at(buf, pos)
    }

    pub(crate) async fn write_at(&mut self, buf: &[u8], pos: u64) -> usize {
        self.capacity_watcher.wait_for_space().await;
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
        self.curr_store
            .blocking_lock()
            .gapless_from_till(pos, last_seek)
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

macro_rules! forward_impl_mut {
    ($v:vis $fn_name:ident, $($param:ident: $t:ty),*; $($returns:ty)?) => {
        impl Store {
            $v fn $fn_name(&mut self, $($param: $t),*) $(-> $returns)? {
                match self {
                    Self::Disk(inner) => inner.$fn_name($($param),*),
                    Self::Mem(inner) => inner.$fn_name($($param),*),
                }
            }
        }
    };
}

forward_impl!(pub(crate) gapless_from_till, pos: u64, last_seek: u64; bool);
forward_impl!(ranges,; RangeSet<u64>);
forward_impl!(last_read_pos,; u64);
forward_impl!(n_supported_ranges,; usize);
forward_impl_mut!(set_range_tx, tx: range_watch::Sender;);
forward_impl_mut!(set_capacity, tx: Capacity;);
forward_impl_mut!(read_blocking_at, buf: &mut [u8], pos: u64; usize);

impl Store {
    /// might not write everything, returns n bytes written
    pub(crate) async fn write_at(&mut self, buf: &[u8], pos: u64) -> usize {
        match self {
            Self::Disk(inner) => inner.write_at(buf, pos).await,
            Self::Mem(inner) => inner.write_at(buf, pos).await,
        }
    }

    fn into_parts(self) -> (range_watch::Sender, Capacity) {
        match self {
            Self::Disk(inner) => inner.into_parts(),
            Self::Mem(inner) => inner.into_parts(),
        }

    }

    fn capacity(&self) -> &Capacity {
        match self {
            Self::Disk(inner) => &inner.capacity,
            Self::Mem(inner) => &inner.capacity,
        }
    }
}
