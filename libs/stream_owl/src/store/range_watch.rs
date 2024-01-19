use derivative::Derivative;
use rangemap::RangeSet;
use std::ops::Range;
use tokio::sync::mpsc;
use tracing::{instrument, trace};

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Receiver {
    #[derivative(Debug = "ignore")]
    rx: mpsc::UnboundedReceiver<Range<u64>>,
    #[derivative(Debug = "ignore")]
    ranges: RangeSet<u64>,
}

#[derive(Clone)]
pub(super) struct Sender {
    tx: mpsc::UnboundedSender<Range<u64>>,
}

// initial range is 0..0
pub(super) fn channel() -> (Sender, Receiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    (
        Sender { tx: tx.clone() },
        Receiver {
            rx,
            ranges: RangeSet::new(),
        },
    )
}

impl Receiver {
    /// blocks till at least one byte is available at `needed_pos`.
    #[instrument(level = "trace")]
    pub(super) async fn wait_for(&mut self, needed_pos: u64) {
        while !self.ranges.contains(&needed_pos) {
            trace!("blocking read until range available");
            match self.rx.recv().await {
                None => {
                    unreachable!("Receiver and Sender should drop at the same time")
                }
                Some(range) => {
                    trace!("new range available: {range:?}");
                    self.ranges.insert(range);
                }
            }
        }
    }
}

impl Sender {
    #[instrument(level = "info", skip(self))]
    pub(super) fn send(&self, range: Range<u64>) {
        if range.is_empty() {
            return
        }

        tracing::trace!("sending new range available: {range:?}");
        if let Err(e) = self.tx.send(range) {
            tracing::warn!("Could not send new range: {e:?}");
        }
    }
}
