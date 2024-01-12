use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use derivative::Derivative;
use tokio::sync::mpsc::{self, Sender};
use tracing::instrument;

use crate::http_client;
use crate::manager::Command;
use crate::network::{Bandwidth, BandwidthAllowed, BandwidthTx};
use crate::reader::{CouldNotCreateRuntime, Reader};
use crate::store::{MigrationHandle, SwitchableStore};

mod builder;
pub use builder::StreamBuilder;
pub use task::StreamDone;
mod task;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error communicating with server")]
    HttpClient(#[from] http_client::Error),
    #[error("Error writing to storage")]
    Writing(std::io::Error),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Id(usize);

impl Id {
    pub(super) fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ManagedHandle {
    /// allows the handle to send a message
    /// to the manager to drop the streams future
    /// or increase/decrease priority.
    #[derivative(Debug = "ignore")]
    cmd_manager: Sender<Command>,
    handle: Handle,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Handle {
    prefetch: usize,
    #[derivative(Debug = "ignore")]
    seek_tx: mpsc::Sender<u64>,
    pause_tx: mpsc::Sender<bool>,
    bandwidth_lim_tx: BandwidthTx,
    is_paused: bool,
    store: SwitchableStore,
    #[derivative(Debug = "ignore")]
    reader_in_use: Arc<Mutex<()>>,
}

#[derive(Debug)]
pub enum GetReaderError {
    ReaderInUse,
    CreationFailed(CouldNotCreateRuntime),
}

macro_rules! managed_async {
    ($fn_name:ident $($param:ident: $t:ty),*$(; $returns:ty)?) => {
        pub async fn $fn_name(&mut self, $($param: $t),*) $(-> $returns)? {
            self.handle.$fn_name($($param),*).await
        }
    };
}

macro_rules! managed {
    ($fn_name:ident $($param:ident: $t:ty),*$(; $returns:ty)?) => {
        pub fn $fn_name(&mut self, $($param: $t),*) $(-> $returns)? {
            self.handle.$fn_name($($param),*)
        }
    };
}

macro_rules! blocking {
    ($name:ident - $new_name:ident $($param:ident: $t:ty),* $(; $ret:ty)?) => {
        /// blocking variant
        ///
        /// # Panics
        ///
        /// This function panics if called within an asynchronous execution
        /// context.
        pub fn $new_name(&mut self, $($param: $t),*) $(-> $ret)? {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(self.$name($($param),*))
        }
    };
}

impl ManagedHandle {
    pub fn set_priority(&mut self, _arg: i32) {
        todo!()
    }

    pub fn id(&self) -> Id {
        todo!()
    }

    managed_async! {pause}
    managed_async! {unpause}
    managed_async! {limit_bandwidth bandwidth: Bandwidth}
    managed_async! {remove_bandwidth_limit}
    managed_async! {use_mem_backend; Option<MigrationHandle>}
    managed_async! {use_disk_backend path: PathBuf; Option<MigrationHandle>}

    managed! {try_get_reader; Result<crate::reader::Reader, GetReaderError>}
    managed! {get_downloaded; ()}
}
impl ManagedHandle {
    blocking! {pause - pause_blocking}
    blocking! {unpause - unpause_blocking}
    blocking! {limit_bandwidth - limit_bandwidth_blocking bandwidth: Bandwidth}
    blocking! {remove_bandwidth_limit - remove_bandwidth_limit_blocking}
    blocking! {use_mem_backend - use_mem_backend_blocking; Option<MigrationHandle>}
    blocking! {use_disk_backend - use_disk_backend_blocking path: PathBuf; Option<MigrationHandle>}
}

impl Drop for ManagedHandle {
    fn drop(&mut self) {
        self.cmd_manager
            .try_send(Command::CancelStream(self.id()))
            .expect("could not cancel stream task when handle was dropped")
    }
}

impl Handle {
    pub async fn limit_bandwidth(&self, bandwidth: Bandwidth) {
        self.bandwidth_lim_tx
            .send(BandwidthAllowed::Limited(bandwidth))
            .await
            .expect("rx is part of Task, which should not drop before Handle");
    }

    pub async fn remove_bandwidth_limit(&self) {
        self.bandwidth_lim_tx
            .send(BandwidthAllowed::UnLimited)
            .await
            .expect("rx is part of Task, which should not drop before Handle");
    }

    pub async fn pause(&mut self) {
        if !self.is_paused {
            self.pause_tx
                .send(true)
                .await
                .expect("rx is part of Task, which should not drop before Handle");
            self.is_paused = true;
        }
    }

    pub async fn unpause(&mut self) {
        if self.is_paused {
            self.pause_tx
                .send(false)
                .await
                .expect("rx is part of Task, which should not drop before Handle");
            self.is_paused = false;
        }
    }

    #[instrument(level = "debug", ret, err(Debug))]
    pub fn try_get_reader(&mut self) -> Result<crate::reader::Reader, GetReaderError> {
        let guard = self
            .reader_in_use
            .try_lock()
            .map_err(|_| GetReaderError::ReaderInUse)?;
        Reader::new(
            guard,
            self.prefetch,
            self.seek_tx.clone(),
            self.store.clone(),
        )
        .map_err(GetReaderError::CreationFailed)
    }

    pub fn get_downloaded(&self) -> () {
        todo!()
    }

    pub async fn use_mem_backend(&mut self) -> Option<MigrationHandle> {
        self.store.to_mem().await
    }

    pub async fn use_disk_backend(&mut self, path: PathBuf) -> Option<MigrationHandle> {
        self.store.to_disk(path).await
    }
}

/// blocking implementations of the async functions above
impl Handle {
    blocking! {pause - pause_blocking}
    blocking! {unpause - unpause_blocking}
    blocking! {limit_bandwidth - limit_bandwidth_blocking bandwidth: Bandwidth}
    blocking! {remove_bandwidth_limit - remove_bandwidth_limit_blocking}
    blocking! {use_mem_backend - use_mem_backend_blocking; Option<MigrationHandle>}
    blocking! {use_disk_backend - use_disk_backend_blocking path: PathBuf; Option<MigrationHandle>}
}

impl Drop for Handle {
    fn drop(&mut self) {
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            rt.block_on(self.store.flush())
        } else {
            self.store.blocking_flush();
        }
    }
}

#[must_use]
pub struct StreamEnded {
    pub(super) res: Result<StreamDone, Error>,
    pub(super) id: Id,
}
