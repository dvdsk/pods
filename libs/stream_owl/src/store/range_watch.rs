use std::ops::Range;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

#[derive(Debug)]
pub(super) struct Receiver {
    rx: broadcast::Receiver<Range<u64>>,
    subscribe_handle: broadcast::Sender<Range<u64>>,
    last: Range<u64>,
}

impl Clone for Receiver {
    fn clone(&self) -> Self {
        Self {
            // critical that subscribe happens before self.last.clone
            // otherwise we could miss a message
            rx: self.subscribe_handle.subscribe(),
            subscribe_handle: self.subscribe_handle.clone(),
            last: self.last.clone(),
        }
    }
}

#[derive(Debug, Clone)]
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
    pub(super) fn blocking_wait_for(&mut self, needed: u64) {
        while self.last.start < needed {
            match self.rx.blocking_recv() {
                Err(RecvError::Closed) => {
                    unreachable!("Receiver and Sender should drop at the same time")
                }
                Err(RecvError::Lagged(_)) => continue,
                Ok(range) => self.last = range,
            }
        }
    }
}

impl Sender {
    pub(super) fn send(&self, range: Range<u64>) {
        let _ignore_no_recievers = self.tx.send(range);
    }
}
