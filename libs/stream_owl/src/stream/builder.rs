use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use std::future::Future;
use std::time::Instant;

use futures::FutureExt;
use tokio::sync::mpsc;

use crate::http_client::Size;
use crate::network::Network;
use crate::store::CapacityBounds;
use crate::store::SwitchableStore;
use crate::{manager, StreamId};

use super::task::Canceld;
use super::{task, Error, Handle, ManagedHandle, StreamEnded};

#[derive(Debug)]
enum StorageChoice {
    Disk(PathBuf),
    Mem(CapacityBounds),
}

#[derive(Debug)]
pub struct StreamBuilder<const STORAGE_SET: bool> {
    url: http::Uri,
    storage: Option<StorageChoice>,
    initial_prefetch: usize,
    restriction: Option<Network>,
}

impl StreamBuilder<false> {
    pub fn new(url: http::Uri) -> StreamBuilder<false> {
        StreamBuilder {
            url,
            storage: None,
            initial_prefetch: 10_000,
            restriction: None,
        }
    }
}

impl StreamBuilder<false> {
    pub fn to_mem(mut self) -> StreamBuilder<true> {
        self.storage = Some(StorageChoice::Mem(CapacityBounds::Unlimited));
        StreamBuilder {
            url: self.url,
            storage: self.storage,
            initial_prefetch: self.initial_prefetch,
            restriction: self.restriction,
        }
    }
    pub fn to_limited_mem(mut self, max_size: NonZeroUsize) -> StreamBuilder<true> {
        let max_size = NonZeroU64::new(max_size.get() as u64)
            .expect("Is already guaranteed to be nonzero");
        self.storage = Some(StorageChoice::Mem(CapacityBounds::Limited(max_size)));
        StreamBuilder {
            url: self.url,
            storage: self.storage,
            initial_prefetch: self.initial_prefetch,
            restriction: self.restriction,
        }
    }
    pub fn to_disk(mut self, path: PathBuf) -> StreamBuilder<true> {
        self.storage = Some(StorageChoice::Disk(path));
        StreamBuilder {
            url: self.url,
            storage: self.storage,
            initial_prefetch: self.initial_prefetch,
            restriction: self.restriction,
        }
    }
}

impl<const STORAGE_SET: bool> StreamBuilder<STORAGE_SET> {
    /// default is 10_000 bytes
    pub fn with_prefetch(mut self, prefetch: usize) -> Self {
        self.initial_prefetch = prefetch;
        self
    }
    pub fn with_network_restriction(mut self, allowed_network: Network) -> Self {
        self.restriction = Some(allowed_network);
        self
    }
}

impl StreamBuilder<true> {
    #[tracing::instrument]
    pub(crate) fn start_managed(
        self,
        manager_tx: mpsc::Sender<manager::Command>,
    ) -> (
        ManagedHandle,
        impl Future<Output = StreamEnded> + Send + 'static,
    ) {
        let id = StreamId::new();
        let (handle, stream_task) = self.start();
        let stream_task = stream_task.map(|res| StreamEnded { res, id });
        let handle = ManagedHandle {
            cmd_manager: manager_tx,
            handle,
        };
        (handle, stream_task)
    }

    #[tracing::instrument]
    pub fn start(
        self,
    ) -> (
        Handle,
        impl Future<Output = Result<Canceld, Error>> + Send + 'static,
    ) {
        let (seek_tx, seek_rx) = mpsc::channel(12);
        let stream_size = Size::default();
        let store = match self.storage.expect("must chose storage option") {
            StorageChoice::Disk(path) => SwitchableStore::new_disk_backed(path, stream_size.clone()),
            StorageChoice::Mem(capacity) => SwitchableStore::new_mem_backed(capacity, stream_size.clone()),
        };

        let handle = Handle {
            created: Instant::now(),
            reader_in_use: Arc::new(Mutex::new(())),
            prefetch: self.initial_prefetch,
            seek_tx,
            store: store.clone(),
        };
        let stream_task = task::new(self.url, store.clone(), seek_rx, self.restriction, stream_size);
        (handle, stream_task)
    }
}
