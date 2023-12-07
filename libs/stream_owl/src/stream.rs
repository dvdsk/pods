use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use derivative::Derivative;
use tokio::sync::mpsc::{self, Sender};
use tracing::instrument;

use crate::http_client;
use crate::manager::Command;
use crate::network::Bandwidth;
use crate::reader::{CouldNotCreateRuntime, Reader};
use crate::store::{MigrationHandle, SwitchableStore};

mod builder;
pub use builder::StreamBuilder;
mod task;
pub use task::Canceld;

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

pub struct ManagedHandle {
    /// allows the handle to send a message
    /// to the manager to drop the streams future
    /// or increase/decrease priority.
    cmd_manager: Sender<Command>,
    handle: Handle,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Handle {
    prefetch: usize,
    #[derivative(Debug = "ignore")]
    seek_tx: mpsc::Sender<u64>,
    store: SwitchableStore,
    created: Instant,
    reader_in_use: Arc<Mutex<()>>,
}

#[derive(Debug)]
pub enum GetReaderError {
    ReaderInUse,
    CreationFailed(CouldNotCreateRuntime),
}

impl ManagedHandle {
    pub fn set_priority(&mut self, _arg: i32) {
        todo!()
    }

    pub fn id(&self) -> Id {
        todo!()
    }

    pub fn limit_bandwidth(&mut self, bandwidth: Bandwidth) {
        self.handle.limit_bandwidth(bandwidth)
    }
    pub fn try_get_reader(&mut self) -> Result<crate::reader::Reader, GetReaderError> {
        self.handle.try_get_reader()
    }
    pub fn get_downloaded(&self) -> () {
        self.handle.get_downloaded()
    }
    pub async fn use_mem_backend(&mut self) -> Option<MigrationHandle> {
        self.handle.use_mem_backend().await
    }
    pub async fn use_disk_backend(&mut self, path: PathBuf) -> Option<MigrationHandle> {
        self.handle.use_disk_backend(path).await
    }
}

impl Handle {
    pub fn limit_bandwidth(&mut self, _bandwidth: Bandwidth) {
        todo!();
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
            self.created.clone(),
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
        self.store.to_disk(&path).await
    }
}

impl Drop for ManagedHandle {
    fn drop(&mut self) {
        self.cmd_manager
            .try_send(Command::CancelStream(self.id()))
            .expect("could not cancel stream task when handle was dropped")
    }
}

#[must_use]
pub struct StreamEnded {
    pub(super) res: Result<Canceld, Error>,
    pub(super) id: Id,
}
