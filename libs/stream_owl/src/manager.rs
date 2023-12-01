use std::path::PathBuf;
use std::str::FromStr;

use futures::Future;
use http::Uri;
use tokio::sync::{mpsc, oneshot};

use crate::network::{Bandwidth, Network};
use crate::stream;

mod task;
pub(crate) use task::Command;

#[derive(Debug)]
pub struct Error;

#[derive(Debug, Default, Clone)]
pub struct ManagerBuilder {
    initial_prefetch: Option<usize>,
    interface: Option<Network>,
}

impl ManagerBuilder {
    pub fn with_prefetch(self, bytes: usize) -> Self {
        Self {
            initial_prefetch: Some(bytes),
            ..self
        }
    }
    pub fn restrict_to_interface(self, network: Network) -> Self {
        Self {
            interface: Some(network),
            ..self
        }
    }
    pub fn build(
        self,
    ) -> (
        Manager,
        impl Future<Output = Error>,
        mpsc::UnboundedReceiver<(stream::Id, stream::Error)>,
    ) {
        Manager::new(self.interface, self.initial_prefetch.unwrap_or(0))
    }
}

pub struct Manager {
    cmd_tx: mpsc::Sender<task::Command>,
}

impl Manager {
    pub fn builder() -> ManagerBuilder {
        ManagerBuilder::default()
    }

    fn new(
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

    /// panics if called from an async context
    pub fn add_stream_to_disk(&mut self, url: &str, path: PathBuf) -> stream::ManagedHandle {
        self.add_stream(url, Some(path))
    }

    /// panics if called from an async context
    pub fn add_stream_to_mem(&mut self, url: &str) -> stream::ManagedHandle {
        self.add_stream(url, None)
    }

    /// panics if called from an async context
    pub fn add_stream(&mut self, url: &str, to_disk: Option<PathBuf>) -> stream::ManagedHandle {
        let url = Uri::from_str(url).unwrap();
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .blocking_send(Command::AddStream {
                url,
                handle_tx: tx,
                to_disk,
            })
            .expect("manager task should still run");
        rx.blocking_recv().unwrap()
    }

    pub fn limit_bandwidth(&mut self, _bandwidth: Bandwidth) {
        todo!();
    }
}
