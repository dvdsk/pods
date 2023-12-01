/// This limits the writer from writing if there is no space. The reader
/// can free up space. Only works under two assumptions, which are currently true:
///  - There is only one reader and writer
///  - Before the writer calls `CapacityWatcher::wait_for_space` the store its
///    writing too has updated the capacity using `Capacity::remove`.
///
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use derivative::Derivative;
use tokio::sync::Notify;
use tracing::instrument;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Bounds {
    Unlimited,
    Limited(NonZeroU64),
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Capacity {
    max: Bounds,
    free: u64,
    #[derivative(Debug = "ignore")]
    write_notify: Arc<Notify>,
    can_write: Arc<AtomicBool>,
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct CapacityWatcher {
    #[derivative(Debug = "ignore")]
    notify: Arc<Notify>,
    can_write: Arc<AtomicBool>,
}

impl CapacityWatcher {
    /// blocks till capacity is available
    /// does not synchronize so only works
    /// with sequential access
    #[tracing::instrument(level = "trace", skip_all)]
    pub(crate) async fn wait_for_space(&self) {
        let notified = self.notify.notified();
        let can_write = self.can_write.load(Ordering::Acquire);
        if can_write {
            return;
        }
        tracing::trace!("waiting for space");
        // if we get notified then space got freed up
        notified.await;
    }
}

impl Capacity {
    pub(crate) fn available(&self) -> usize {
        // cast to smaller unsigned saturates at upper bound
        if let Bounds::Unlimited = self.max {
            return usize::MAX;
        };

        self.free as usize
    }

    pub(crate) fn total(&self) -> Bounds {
        self.max
    }

    #[instrument(level = "trace", skip(self), fields(self.free = self.free))]
    pub(crate) fn add(&mut self, amount: usize) {
        self.free += amount as u64;
        if self.available() > 0 {
            tracing::trace!("store has capacity again");
            self.can_write.store(true, Ordering::Release);
            self.write_notify.notify_one()
        }
    }

    #[instrument(level = "trace", skip(self), fields(self.free = self.free))]
    pub(crate) fn remove(&mut self, amount: usize) {
        self.free = self.free.saturating_sub(amount as u64);
        if self.available() == 0 {
            tracing::trace!("store is out of capacity");
            self.can_write.store(false, Ordering::Release);
        }
    }
}

#[instrument(level = "debug", ret)]
pub(crate) fn new(max: Bounds) -> (CapacityWatcher, Capacity) {
    let notify = Arc::new(Notify::new());
    let can_write = Arc::new(AtomicBool::new(true));
    (
        CapacityWatcher {
            notify: notify.clone(),
            can_write: can_write.clone(),
        },
        match max {
            Bounds::Limited(bytes) => Capacity {
                max,
                free: bytes.get(),
                write_notify: notify,
                can_write,
            },
            Bounds::Unlimited => Capacity {
                max,
                free: u64::MAX,
                write_notify: notify,
                can_write,
            },
        },
    )
}
