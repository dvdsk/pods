use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use futures::{pin_mut, FutureExt};
use futures_concurrency::future::Race;
use std::future::Future;
use tokio::sync::mpsc;
use tracing::debug;

use crate::http_client::Size;
use crate::network::{Bandwidth, Network, BandwidthAllowed, BandwidthLim};
use crate::store::SwitchableStore;
use crate::{manager, StreamDone, StreamId};

use super::{task, Error, Handle, ManagedHandle, StreamEnded};

#[derive(Debug)]
enum StorageChoice {
    Disk(PathBuf),
    MemLimited(NonZeroUsize),
    MemUnlimited,
}

#[derive(Debug)]
pub struct StreamBuilder<const STORAGE_SET: bool> {
    url: http::Uri,
    storage: Option<StorageChoice>,
    initial_prefetch: usize,
    restriction: Option<Network>,
    start_paused: bool,
    bandwidth: BandwidthAllowed,
}

impl StreamBuilder<false> {
    pub fn new(url: http::Uri) -> StreamBuilder<false> {
        StreamBuilder {
            url,
            storage: None,
            initial_prefetch: 10_000,
            restriction: None,
            start_paused: false,
            bandwidth: BandwidthAllowed::UnLimited,
        }
    }
}

impl StreamBuilder<false> {
    pub fn to_unlimited_mem(mut self) -> StreamBuilder<true> {
        self.storage = Some(StorageChoice::MemUnlimited);
        StreamBuilder {
            url: self.url,
            storage: self.storage,
            initial_prefetch: self.initial_prefetch,
            restriction: self.restriction,
            start_paused: self.start_paused,
            bandwidth: self.bandwidth,
        }
    }
    pub fn to_limited_mem(mut self, max_size: NonZeroUsize) -> StreamBuilder<true> {
        self.storage = Some(StorageChoice::MemLimited(max_size));
        StreamBuilder {
            url: self.url,
            storage: self.storage,
            initial_prefetch: self.initial_prefetch,
            restriction: self.restriction,
            start_paused: self.start_paused,
            bandwidth: self.bandwidth,
        }
    }
    pub fn to_disk(mut self, path: PathBuf) -> StreamBuilder<true> {
        self.storage = Some(StorageChoice::Disk(path));
        StreamBuilder {
            url: self.url,
            storage: self.storage,
            initial_prefetch: self.initial_prefetch,
            restriction: self.restriction,
            start_paused: self.start_paused,
            bandwidth: self.bandwidth,
        }
    }
}

impl<const STORAGE_SET: bool> StreamBuilder<STORAGE_SET> {
    pub fn start_paused(mut self, start_paused: bool) -> Self {
        self.start_paused = start_paused;
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
    pub fn with_bandwidth_limit(mut self, bandwidth: Bandwidth) -> Self {
        self.bandwidth = BandwidthAllowed::Limited(bandwidth);
        self
    }
}

impl StreamBuilder<true> {
    #[tracing::instrument]
    pub(crate) async fn start_managed(
        self,
        manager_tx: mpsc::Sender<manager::Command>,
    ) -> Result<
        (
            ManagedHandle,
            impl Future<Output = StreamEnded> + Send + 'static,
        ),
        crate::store::Error,
    > {
        let id = StreamId::new();
        let (handle, stream_task) = self.start().await?;
        let stream_task = stream_task.map(|res| StreamEnded { res, id });
        let handle = ManagedHandle {
            cmd_manager: manager_tx,
            handle,
        };
        Ok((handle, stream_task))
    }

    #[tracing::instrument]
    pub async fn start(
        self,
    ) -> Result<
        (
            Handle,
            impl Future<Output = Result<StreamDone, Error>> + Send + 'static,
        ),
        crate::store::Error,
    > {
        let stream_size = Size::default();
        let store = match self.storage.expect("must chose storage option") {
            StorageChoice::Disk(path) => {
                SwitchableStore::new_disk_backed(path, stream_size.clone()).await
            }
            StorageChoice::MemLimited(limit) => {
                SwitchableStore::new_limited_mem_backed(limit, stream_size.clone())
            }
            StorageChoice::MemUnlimited => {
                SwitchableStore::new_unlimited_mem_backed(stream_size.clone())
            }
        }?;

        let (seek_tx, seek_rx) = mpsc::channel(12);
        let (pause_tx, pause_rx) = mpsc::channel(12);
        let (bandwidth_lim, bandwidth_lim_tx) = BandwidthLim::new(self.bandwidth);

        let mut handle = Handle {
            reader_in_use: Arc::new(Mutex::new(())),
            prefetch: self.initial_prefetch,
            seek_tx,
            is_paused: false,
            store: store.clone(),
            pause_tx,
            bandwidth_lim_tx,
        };

        if self.start_paused {
            handle.pause().await;
        }

        let stream_task = task::new(
            self.url,
            store.clone(),
            seek_rx,
            self.restriction,
            bandwidth_lim,
            stream_size,
        );
        let stream_task = pausable(stream_task, pause_rx);
        Ok((handle, stream_task))
    }
}

async fn pausable<F>(task: F, mut pause_rx: mpsc::Receiver<bool>) -> Result<StreamDone, Error>
where
    F: Future<Output = Result<StreamDone, Error>>,
{
    enum Res {
        Pause(Option<bool>),
        Task(Result<StreamDone, Error>),
    }

    pin_mut!(task);
    loop {
        let get_pause = pause_rx.recv().map(Res::Pause);
        let task_ends = task.as_mut().map(Res::Task);
        match (get_pause, task_ends).race().await {
            Res::Pause(None) => return Ok(StreamDone::Canceld),
            Res::Pause(Some(true)) => loop {
                match pause_rx.recv().await {
                    Some(true) => debug!("already paused"),
                    Some(false) => break,
                    None => return Ok(StreamDone::Canceld),
                }
            },
            Res::Pause(Some(false)) => debug!("not paused"),
            Res::Task(result) => return result,
        }
    }
}
