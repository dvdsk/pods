use derivative::Derivative;
use futures::FutureExt;
use futures_concurrency::future::Race;
use rangemap::RangeSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

mod capacity;
mod disk;
mod mem;
mod migrate;
mod range_watch;

pub(crate) use capacity::Bounds as CapacityBounds;
pub use migrate::{MigrationError, MigrationHandle};

use capacity::CapacityWatcher;

use crate::http_client::Size;

use self::capacity::Capacity;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub(crate) struct SwitchableStore {
    curr_store: Arc<Mutex<Store>>,
    curr_range: range_watch::Receiver,
    capacity_watcher: CapacityWatcher,
    stream_size: Size,
}

#[derive(Debug)]
pub(crate) enum Store {
    Disk(disk::Disk),
    Mem(mem::Memory),
}

#[derive(Debug, Clone)]
pub(super) enum StoreVariant {
    Disk,
    Mem,
}

impl SwitchableStore {
    #[tracing::instrument]
    pub(crate) fn new_disk_backed(path: PathBuf, stream_size: Size) -> Self {
        let (capacity_watcher, capacity) = capacity::new(capacity::Bounds::Unlimited);
        let (tx, rx) = range_watch::channel();
        let disk = disk::Disk::new(&path, capacity, tx).unwrap();
        Self {
            curr_range: rx,
            capacity_watcher,
            curr_store: Arc::new(Mutex::new(Store::Disk(disk))),
            stream_size,
        }
    }

    #[tracing::instrument]
    pub(crate) fn new_mem_backed(max_cap: capacity::Bounds, stream_size: Size) -> Self {
        let (capacity_watcher, capacity) = capacity::new(max_cap);
        let (tx, rx) = range_watch::channel();
        let mem = mem::Memory::new(capacity, tx).unwrap();
        Self {
            curr_range: rx,
            capacity_watcher,
            curr_store: Arc::new(Mutex::new(Store::Mem(mem))),
            stream_size,
        }
    }

    pub(crate) async fn variant(&self) -> StoreVariant {
        match *self.curr_store.lock().await {
            Store::Disk(_) => StoreVariant::Disk,
            Store::Mem(_) => StoreVariant::Mem,
        }
    }

    /// Returns number of bytes read, 0 means end of file.
    /// This reads as soon as bytes are available.
    #[tracing::instrument(level = "trace", skip(buf), fields(buf_len = buf.len()), ret)]
    pub(super) async fn read_at(&mut self, buf: &mut [u8], pos: u64) -> ReadResult {
        enum Res {
            RangeReady,
            PosBeyondEOF,
        }

        let Self {
            curr_range,
            stream_size,
            ..
        } = self;
        let wait_for_range = curr_range.wait_for(pos).map(|_| Res::RangeReady);
        let watch_eof_pos = stream_size.eof_smaller_then(pos).map(|_| Res::PosBeyondEOF);
        let res = (wait_for_range, watch_eof_pos).race().await;

        if let Res::PosBeyondEOF = res {
            ReadResult::EndOfStream
        } else {
            let n_read = self.curr_store.lock().await.read_at(buf, pos).await;
            ReadResult::ReadN(n_read)
        }
    }

    pub(crate) async fn write_at(&mut self, buf: &[u8], pos: u64) -> usize {
        self.capacity_watcher.wait_for_space().await;
        match &mut *self.curr_store.lock().await {
            Store::Disk(inner) => inner.write_at(buf, pos).await,
            Store::Mem(inner) => inner.write_at(buf, pos).await,
        }
    }

    /// refers to the size of the stream if it was complete
    pub(crate) fn size(&self) -> Size {
        self.stream_size.clone()
    }

    pub(crate) fn gapless_from_till(&self, last_seek: u64, pos: u64) -> bool {
        self.curr_store
            .blocking_lock()
            .gapless_from_till(pos, last_seek)
    }
}

#[derive(Debug)]
pub(crate) enum ReadResult {
    EndOfStream,
    ReadN(usize),
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

    ($v:vis async $fn_name:ident, $($param:ident: $t:ty),*; $($returns:ty)?) => {
        impl Store {
            $v async fn $fn_name(&mut self, $($param: $t),*) $(-> $returns)? {
                match self {
                    Self::Disk(inner) => inner.$fn_name($($param),*).await,
                    Self::Mem(inner) => inner.$fn_name($($param),*).await,
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
forward_impl_mut!(async read_at, buf: &mut [u8], pos: u64; usize);
forward_impl_mut!(pub(crate) async write_at, buf: &[u8], pos: u64; usize);

impl Store {
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
