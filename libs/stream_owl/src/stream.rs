use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures::{Future, FutureExt};
use tokio::sync::mpsc::{self, Sender};

use crate::http_client;
use crate::manager::Command;
use crate::network::{Bandwith, Network};
use crate::reader::Reader;
use crate::store::{MigrationHandle, SwitchableStore};

use self::task::Canceld;

mod task;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error communicating with server")]
    HttpClient(#[from] http_client::Error),
    #[error("Error writing to strorage")]
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

pub struct Handle {
    prefetch: usize,
    seek_tx: mpsc::Sender<u64>,
    store: SwitchableStore,
    cmd_tx: Sender<Command>,
    reader_in_use: Arc<Mutex<()>>,
}

#[derive(Debug, Clone)]
pub struct ReaderInUse;

impl Handle {
    pub fn set_priority(&mut self, _arg: i32) {
        todo!()
    }

    pub fn limit_bandwith(&mut self, _bandwith: Bandwith) {
        todo!();
    }

    pub fn try_get_reader(&mut self) -> Result<crate::reader::Reader, ReaderInUse> {
        let guard = self.reader_in_use.try_lock().map_err(|_| ReaderInUse)?;
        Ok(Reader::new(
            guard,
            self.prefetch,
            self.seek_tx.clone(),
            self.store.clone(),
        ))
    }

    pub fn get_downloaded(&self) -> () {
        todo!()
    }

    pub fn id(&self) -> Id {
        todo!()
    }

    pub async fn use_mem_backend(&mut self) -> Option<MigrationHandle> {
        self.store.to_mem().await
    }

    pub async fn use_disk_backend(&mut self, path: PathBuf) -> Option<MigrationHandle> {
        self.store.to_disk(&path).await
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.cmd_tx // tell the manager task to abort the task
            .try_send(Command::CancelStream(self.id()))
            .expect("could not cancel stream task when handle was dropped")
    }
}

#[must_use]
pub struct StreamEnded {
    pub(super) res: Result<Canceld, Error>,
    pub(super) id: Id,
}

pub(crate) fn new(
    url: http::Uri,
    to_disk: Option<PathBuf>,
    cmd_tx: Sender<Command>,
    initial_prefetch: usize,
    id: Id,
    restriction: Option<Network>,
) -> (Handle, impl Future<Output = StreamEnded> + Send + 'static) {
    let (seek_tx, seek_rx) = mpsc::channel(12);
    let store = match to_disk {
        Some(path) => crate::store::SwitchableStore::new_disk_backed(path),
        None => crate::store::SwitchableStore::new_mem_backed(),
    };

    (
        Handle {
            cmd_tx,
            reader_in_use: Arc::new(Mutex::new(())),
            prefetch: initial_prefetch,
            seek_tx,
            store: store.clone(),
        },
        task::new(url, store.clone(), seek_rx, restriction).map(|res| StreamEnded { res, id }),
    )
}
