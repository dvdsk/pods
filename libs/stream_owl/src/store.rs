use derivative::Derivative;
use futures::FutureExt;
use futures_concurrency::future::Race;
use rangemap::RangeSet;
use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

mod capacity;
mod disk;
mod limited_mem;
mod migrate;
mod range_watch;
mod unlimited_mem;

pub(crate) use capacity::Bounds as CapacityBounds;
pub use migrate::{MigrationError, MigrationHandle};

use capacity::CapacityWatcher;

use crate::http_client::Size;

use self::capacity::Capacity;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub(crate) struct SwitchableStore {
    pub(crate) curr_store: Arc<Mutex<Store>>,
    curr_range: range_watch::Receiver,
    capacity_watcher: CapacityWatcher,
    stream_size: Size,
}

#[derive(Debug)]
pub(crate) enum Store {
    Disk(disk::Disk),
    MemLimited(limited_mem::Memory),
    MemUnlimited(unlimited_mem::Memory),
}

#[derive(Debug, Clone)]
pub(super) enum StoreVariant {
    Disk,
    MemLimited,
    MemUnlimited,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Not critical
    #[error("Refusing write while in the middle of a seek")]
    SeekInProgress,
    #[error("Error in memory backend")]
    MemoryLimited(#[from] limited_mem::Error),
    #[error("Error in memory backend")]
    MemoryUnlimited(#[from] unlimited_mem::Error),
    #[error("Error in disk backend")]
    Disk(#[from] disk::Error),
}

impl SwitchableStore {
    #[tracing::instrument]
    pub(crate) async fn new_disk_backed(path: PathBuf, stream_size: Size) -> Result<Self, Error> {
        let (capacity_watcher, capacity) = capacity::new(capacity::Bounds::Unlimited);
        let (tx, rx) = range_watch::channel();
        let disk = disk::Disk::new(path, capacity, tx).await?;
        Ok(Self {
            curr_range: rx,
            capacity_watcher,
            curr_store: Arc::new(Mutex::new(Store::Disk(disk))),
            stream_size,
        })
    }

    #[tracing::instrument]
    pub(crate) fn new_limited_mem_backed(
        max_cap: NonZeroUsize,
        stream_size: Size,
    ) -> Result<Self, Error> {
        let max_cap = NonZeroU64::new(max_cap.get() as u64).expect("already nonzero");
        let (capacity_watcher, capacity) = capacity::new(CapacityBounds::Limited(max_cap));
        let (tx, rx) = range_watch::channel();
        let mem = limited_mem::Memory::new(capacity, tx)?;
        Ok(Self {
            curr_range: rx,
            capacity_watcher,
            curr_store: Arc::new(Mutex::new(Store::MemLimited(mem))),
            stream_size,
        })
    }

    #[tracing::instrument]
    pub(crate) fn new_unlimited_mem_backed(stream_size: Size) -> Result<Self, Error> {
        let (capacity_watcher, capacity) = capacity::new(CapacityBounds::Unlimited);
        let (tx, rx) = range_watch::channel();
        let mem = unlimited_mem::Memory::new(capacity, tx)?;
        Ok(Self {
            curr_range: rx,
            capacity_watcher,
            curr_store: Arc::new(Mutex::new(Store::MemUnlimited(mem))),
            stream_size,
        })
    }

    pub(crate) async fn variant(&self) -> StoreVariant {
        match *self.curr_store.lock().await {
            Store::Disk(_) => StoreVariant::Disk,
            Store::MemLimited(_) => StoreVariant::MemLimited,
            Store::MemUnlimited(_) => StoreVariant::MemUnlimited,
        }
    }

    /// Returns number of bytes read, 0 means end of file.
    /// This reads as soon as bytes are available.
    #[tracing::instrument(level = "trace", skip(buf), fields(buf_len = buf.len()), ret)]
    pub(super) async fn read_at(&mut self, buf: &mut [u8], pos: u64) -> Result<usize, ReadError> {
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
            Err(ReadError::EndOfStream)
        } else {
            let n_read = self.curr_store.lock().await.read_at(buf, pos).await?;
            Ok(n_read)
        }
    }

    #[instrument(level = "trace", skip(self, buf))]
    pub(crate) async fn write_at(&self, buf: &[u8], pos: u64) -> Result<NonZeroUsize, Error> {
        self.capacity_watcher.wait_for_space().await;
        self.curr_store.lock().await.write_at(buf, pos).await
    }

    /// refers to the size of the stream if it was complete
    pub(crate) fn size(&self) -> Size {
        self.stream_size.clone()
    }

    pub(crate) async fn flush(&self) -> Result<(), Error> {
        self.curr_store.lock().await.flush().await
    }
}

#[derive(Debug)]
pub(crate) struct SeekInProgress;

#[derive(thiserror::Error, Debug)]
pub(crate) enum ReadError {
    #[error(transparent)]
    Store(#[from] Error),
    #[error("End of stream reached")]
    EndOfStream,
}

macro_rules! forward_impl {
    ($v:vis $fn_name:ident, $($param:ident: $t:ty),*; $returns:ty) => {
        impl Store {
            $v fn $fn_name(&self, $($param: $t),*) -> $returns {
                match self {
                    Self::Disk(inner) => inner.$fn_name($($param),*),
                    Self::MemUnlimited(inner) => inner.$fn_name($($param),*),
                    Self::MemLimited(inner) => inner.$fn_name($($param),*),
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
                    Self::MemUnlimited(inner) => inner.$fn_name($($param),*),
                    Self::MemLimited(inner) => inner.$fn_name($($param),*),
                }
            }
        }
    };

    ($v:vis async $fn_name:ident, $($param:ident: $t:ty),*; $($returns:ty)?) => {
        impl Store {
            $v async fn $fn_name(&mut self, $($param: $t),*) $(-> $returns)? {
                match self {
                    Self::Disk(inner) => inner.$fn_name($($param),*).await,
                    Self::MemUnlimited(inner) => inner.$fn_name($($param),*).await,
                    Self::MemLimited(inner) => inner.$fn_name($($param),*).await,
                }
            }
        }
    };
}

forward_impl!(pub(crate) gapless_from_till, pos: u64, last_seek: u64; bool);
forward_impl!(pub(crate) ranges,; RangeSet<u64>);
forward_impl!(last_read_pos,; u64);
forward_impl!(n_supported_ranges,; usize);
forward_impl_mut!(pub(crate) writer_jump, to_pos: u64;);
forward_impl_mut!(set_range_tx, tx: range_watch::Sender;);
forward_impl_mut!(set_capacity, tx: Capacity;);

impl Store {
    pub(crate) async fn write_at(&mut self, buf: &[u8], pos: u64) -> Result<NonZeroUsize, Error> {
        match self {
            Store::Disk(inner) => inner.write_at(buf, pos).await.map_err(Error::Disk),
            Store::MemLimited(inner) => inner.write_at(buf, pos).await.map_err(|e| match e {
                limited_mem::Error::SeekInProgress => Error::SeekInProgress,
                other => Error::MemoryLimited(other),
            }),
            Store::MemUnlimited(inner) => inner.write_at(buf, pos).await.map_err(|e| match e {
                unlimited_mem::Error::SeekInProgress => Error::SeekInProgress,
                other => Error::MemoryUnlimited(other),
            }),
        }
    }
    async fn read_at(&mut self, buf: &mut [u8], pos: u64) -> Result<usize, Error> {
        match self {
            Self::Disk(inner) => inner.read_at(buf, pos).await.map_err(Error::Disk),
            Self::MemLimited(inner) => Ok(inner.read_at(buf, pos)),
            Self::MemUnlimited(inner) => Ok(inner.read_at(buf, pos)),
        }
    }

    fn into_parts(self) -> (range_watch::Sender, Capacity) {
        match self {
            Self::Disk(inner) => inner.into_parts(),
            Self::MemLimited(inner) => inner.into_parts(),
            Self::MemUnlimited(inner) => inner.into_parts(),
        }
    }

    fn capacity(&self) -> &Capacity {
        match self {
            Self::Disk(inner) => &inner.capacity,
            Self::MemLimited(inner) => &inner.capacity,
            Self::MemUnlimited(inner) => &inner.capacity,
        }
    }
}
