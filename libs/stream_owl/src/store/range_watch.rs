use derivative::Derivative;
use std::ops::Range;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tracing::{instrument, trace};

#[derive(Derivative)]
#[derivative(Debug)]
pub(super) struct Receiver {
    #[derivative(Debug = "ignore")]
    rx: broadcast::Receiver<Range<u64>>,
    #[derivative(Debug = "ignore")]
    subscribe_handle: broadcast::Sender<Range<u64>>,
    last: Range<u64>,
}

impl Clone for Receiver {
    fn clone(&self) -> Self {
        Self {
            // field order matters! its critical that subscribe happens before
            // self.last.clone otherwise we could miss a message
            rx: self.subscribe_handle.subscribe(),
            subscribe_handle: self.subscribe_handle.clone(),
            last: self.last.clone(),
        }
    }
}

#[derive(Clone)]
pub(super) struct Sender {
    tx: broadcast::Sender<Range<u64>>,
}

// initial range is 0..0
pub(super) fn channel() -> (Sender, Receiver) {
    let (tx, rx) = broadcast::channel(16);
    (
        Sender { tx: tx.clone() },
        Receiver {
            rx,
            subscribe_handle: tx,
            last: 0..0,
        },
    )
}

impl Receiver {
    /// blocks till at least one byte is available at `needed_pos`.
    #[instrument(level = "trace")]
    pub(super) async fn wait_for(&mut self, needed_pos: u64) {
        while self.last.end < needed_pos + 1 {
            trace!("blocking until range is available");
            match self.rx.recv().await {
                Err(RecvError::Closed) => {
                    unreachable!("Receiver and Sender should drop at the same time")
                }
                Err(RecvError::Lagged(_)) => continue,
                Ok(range) => {
                    trace!("new range available: {range:?}");
                    self.last = range;
                }
            }
        }
    }
}

impl Sender {
    #[instrument(level = "info", skip(self))]
    pub(super) fn send(&self, range: Range<u64>) {
        tracing::trace!("sending new range available: {range:?}");
        if let Err(e) = self.tx.send(range) {
            tracing::warn!("Could not send new range: {e:?}");
        }
    }
}