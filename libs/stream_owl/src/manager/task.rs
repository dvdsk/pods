use std::collections::HashMap;
use std::path::PathBuf;

use futures::FutureExt;
use futures_concurrency::future::Race;
use tokio::sync::mpsc::{error::SendError, Receiver, Sender, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::{AbortHandle, JoinError, JoinSet};
use tracing::{trace, warn};

use crate::network::Network;
use crate::stream::{StreamBuilder, StreamEnded};
use crate::{stream, StreamError, StreamId};

pub(crate) enum Command {
    AddStream {
        url: hyper::Uri,
        handle_tx: oneshot::Sender<stream::ManagedHandle>,
        /// if non use memory store
        to_disk: Option<PathBuf>,
    },
    CancelStream(StreamId),
}

enum Res {
    StreamComplete { id: StreamId },
    StreamError { id: StreamId, error: StreamError },
    NewCmd(Command),
    Dropped,
}

impl From<Option<Command>> for Res {
    fn from(value: Option<Command>) -> Self {
        match value {
            Some(cmd) => Res::NewCmd(cmd),
            None => Res::Dropped,
        }
    }
}

impl From<Option<Result<StreamEnded, JoinError>>> for Res {
    fn from(value: Option<Result<StreamEnded, JoinError>>) -> Self {
        let StreamEnded { id, res } = value
            .expect("streams JoinSet should never be empty")
            .expect("stream should never panic");
        if let Err(error) = res {
            Res::StreamError { id, error }
        } else {
            Res::StreamComplete { id }
        }
    }
}

async fn wait_forever() -> StreamEnded {
    futures::pending!();
    unreachable!()
}

pub(super) async fn run(
    cmd_tx: Sender<Command>,
    mut cmd_rx: Receiver<Command>,
    err_tx: UnboundedSender<(StreamId, StreamError)>,
    restriction: Option<Network>,
    initial_prefetch: usize,
) -> super::Error {
    use Command::*;

    let mut streams = JoinSet::new();
    streams.spawn(wait_forever());
    let mut abort_handles = HashMap::new();

    loop {
        let new_cmd = cmd_rx.recv().map(Res::from);
        let stream_err = streams.join_next().map(Res::from);

        match (new_cmd, stream_err).race().await {
            Res::NewCmd(AddStream {
                url,
                handle_tx,
                to_disk,
            }) => add_stream(
                &mut streams,
                &mut abort_handles,
                url,
                to_disk,
                handle_tx,
                cmd_tx.clone(),
                restriction.clone(),
                initial_prefetch,
            ),
            Res::NewCmd(CancelStream(id)) => {
                if let Some(handle) = abort_handles.remove(&id) {
                    handle.abort();
                }
            }
            Res::StreamError { id, error } => {
                if let Err(SendError((id, error))) = err_tx.send((id, error)) {
                    warn!("stream {id:?} ran into an error, it could not be send to API user as the error stream recieve part has been dropped. Error was: {error:?}")
                }
            }
            Res::StreamComplete { .. } | Res::Dropped => (),
        }
    }
}

fn add_stream(
    streams: &mut JoinSet<stream::StreamEnded>,
    abort_handles: &mut HashMap<StreamId, AbortHandle>,
    url: http::Uri,
    to_disk: Option<PathBuf>,
    handle_tx: oneshot::Sender<stream::ManagedHandle>,
    cmd_tx: Sender<Command>,
    restriction: Option<Network>,
    initial_prefetch: usize,
) {
    let mut stream = StreamBuilder::new(url);
    if let Some(path) = to_disk {
        stream = stream.to_disk(path);
    }
    if let Some(allowed) = restriction {
        stream = stream.with_network_restriction(allowed)
    }
    stream = stream.with_prefetch(initial_prefetch);

    let (handle, stream_task) = stream.start_managed(cmd_tx);

    let abort_handle = streams.spawn(stream_task);
    abort_handles.insert(handle.id(), abort_handle);
    if let Err(_) = handle_tx.send(handle) {
        trace!("add_stream canceld on user side");
        // dropping the handle here will cancel the streams task
    }
}
