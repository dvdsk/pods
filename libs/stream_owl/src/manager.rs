use std::str::FromStr;

use futures::Future;
use http::Uri;
use tokio::sync::{mpsc, oneshot};

use crate::network::{Bandwith, Network};
use crate::stream;

mod task;
pub(crate) use task::Command;

#[derive(Debug)]
pub struct Error;

pub struct Manager {
    cmd_tx: mpsc::Sender<task::Command>,
}

impl Manager {
    pub fn new(
        initial_prefetch: usize,
    ) -> (
        Self,
        impl Future<Output = Error>,
        mpsc::UnboundedReceiver<(stream::Id, stream::Error)>,
    ) {
        Self::new_inner(None, initial_prefetch)
    }

    pub fn new_restricted(
        interface: Network,
        initial_prefetch: usize,
    ) -> (
        Self,
        impl Future<Output = Error>,
        mpsc::UnboundedReceiver<(stream::Id, stream::Error)>,
    ) {
        Self::new_inner(Some(interface), initial_prefetch)
    }

    fn new_inner(
        restriction: Option<Network>,
        initial_prefetch: usize,
    ) -> (
        Self,
        impl Future<Output = Error>,
        mpsc::UnboundedReceiver<(stream::Id, stream::Error)>,
    ) {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (err_tx, err_rx) = mpsc::unbounded_channel();
        (
            Self {
                cmd_tx: cmd_tx.clone(),
            },
            task::run(cmd_tx, cmd_rx, err_tx, restriction, initial_prefetch),
            err_rx,
        )
    }

    pub fn add_stream_to_disk(&mut self, url: &str) -> stream::Handle {
        self.add_stream(url, true)
    }

    pub fn add_stream_to_mem(&mut self, url: &str) -> stream::Handle {
        self.add_stream(url, false)
    }

    pub fn add_stream(&mut self, url: &str, _to_disk: bool) -> stream::Handle {
        let url = Uri::from_str(url).unwrap();
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .blocking_send(Command::AddStream { url, handle_tx: tx })
            .expect("manager task should still run");
        rx.blocking_recv().unwrap()
    }

    pub fn limit_bandwith(&mut self, _bandwith: Bandwith) {
        todo!();
    }
}
