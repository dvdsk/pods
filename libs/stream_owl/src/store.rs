use derivative::Derivative;
use futures::FutureExt;
use futures_concurrency::future::Race;
use rangemap::RangeSet;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

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
    pub(crate) curr_store: Arc<Mutex<Store>>,
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

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Not critical
    #[error("Refusing write while in the middle of a seek")]
    SeekInProgress,
    #[error("Error in memory backend")]
    Memory(mem::Error),
    #[error("Error in disk backend")]
    Disk(#[from] disk::Error),
}

impl SwitchableStore {
    #[tracing::instrument]
    pub(crate) async fn new_disk_backed(path: PathBuf, stream_size: Size) -> Self {
        let (capacity_watcher, capacity) = capacity::new(capacity::Bounds::Unlimited);
        let (tx, rx) = range_watch::channel();
        let disk = disk::Disk::new(path, capacity, tx).await.unwrap();
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
            Store::Mem(inner) => inner.write_at(buf, pos).await.map_err(|e| match e {
                mem::Error::SeekInProgress => Error::SeekInProgress,
                other => Error::Memory(other),
            }),
        }
    }
    async fn read_at(&mut self, buf: &mut [u8], pos: u64) -> Result<usize, Error> {
        match self {
            Self::Disk(inner) => inner.read_at(buf, pos).await.map_err(Error::Disk),
            Self::Mem(inner) => Ok(inner
                .read_at(buf, pos)
                .await
                .expect("never runs into an error")),
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
