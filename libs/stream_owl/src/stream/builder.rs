use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use std::future::Future;

use futures::FutureExt;
use tokio::sync::mpsc;

use crate::network::Network;
use crate::{manager, StreamId};

use super::task::Canceld;
use super::{task, Error, Handle, ManagedHandle, StreamEnded};

pub struct StreamBuilder {
    url: http::Uri,
    to_disk: Option<PathBuf>,
    initial_prefetch: usize,
    restriction: Option<Network>,
}

impl StreamBuilder {
    pub fn new(url: http::Uri) -> Self {
        Self {
            url,
            to_disk: None,
            initial_prefetch: 10_000,
            restriction: None,
        }
    }
    pub fn to_disk(mut self, path: PathBuf) -> Self {
        self.to_disk = Some(path);
        self
    }
    /// default is 10_000 bytes
    pub fn with_prefetch(mut self, prefetch: usize) -> Self {
        self.initial_prefetch = prefetch;
        self
    }
    pub fn with_network_restriction(mut self, allowed_network: Network) -> Self {
        self.restriction = Some(allowed_network);
        self
    }

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

    pub fn start(
        self,
    ) -> (
        Handle,
        impl Future<Output = Result<Canceld, Error>> + Send + 'static,
    ) {
        let (seek_tx, seek_rx) = mpsc::channel(12);
        let store = match self.to_disk {
            Some(path) => crate::store::SwitchableStore::new_disk_backed(path),
            None => crate::store::SwitchableStore::new_mem_backed(),
        };

        let handle = Handle {
            reader_in_use: Arc::new(Mutex::new(())),
            prefetch: self.initial_prefetch,
            seek_tx,
            store: store.clone(),
        };
        let stream_task = task::new(self.url, store.clone(), seek_rx, self.restriction);
        (handle, stream_task)
    }
}
