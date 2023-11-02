use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use futures::{Future, FutureExt};
use tokio::sync::mpsc::{self, Sender};

use crate::http_client;
use crate::manager::Command;
use crate::network::{Bandwith, Network};
use crate::reader::Reader;

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
    cmd_tx: Sender<Command>,
    reader: Reader,
    reader_in_use: Arc<AtomicBool>,
}

impl Handle {
    pub fn set_priority(&mut self, _arg: i32) {
        todo!()
    }

    pub fn limit_bandwith(&mut self, _bandwith: Bandwith) {
        todo!();
    }

    pub fn try_get_reader(&mut self) -> Result<crate::reader::Reader, ()> {
        todo!()
    }

    pub fn get_downloaded(&self) -> () {
        todo!()
    }

    pub fn id(&self) -> Id {
        todo!()
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
    cmd_tx: Sender<Command>,
    initial_prefetch: usize,
    id: Id,
    restriction: Option<Network>,
) -> (Handle, impl Future<Output = StreamEnded> + Send + 'static) {
    let (seek_tx, seek_rx) = mpsc::channel(12);
    let reader = Reader { seek_tx, prefetch: initial_prefetch };
    (
        Handle {
            cmd_tx,
            reader,
            reader_in_use: Arc::new(AtomicBool::new(false)),
        },
        task::new(url, seek_rx, restriction).map(|res| StreamEnded { res, id }),
    )
}
