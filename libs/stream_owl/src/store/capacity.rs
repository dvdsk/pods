use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Notify;

#[derive(Debug)]
pub struct Capacity {
    total: Option<NonZeroU64>,
    free: u64,
    write_notify: Arc<Notify>,
    can_write: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct CapacityWatcher {
    notify: Arc<Notify>,
    can_write: Arc<AtomicBool>,
}

impl CapacityWatcher {
    /// blocks till capacity is availible
    /// does not synchronize so only works
    /// with sequential access
    #[tracing::instrument(level="trace", skip_all, ret)]
    pub(crate) async fn wait_for_space(&self) {
        let notified = self.notify.notified();
        if self.can_write.load(Ordering::Acquire) {
            return;
        } else {
            notified.await;
        }
    }
}

impl Capacity {
    pub(crate) fn availible(&self) -> usize {
        // cast to smaller unsigned saturates at upper bound
        let Some(limited_total) = self.total else {
            return usize::MAX;
        };

        (limited_total.get() - self.free) as usize
    }

    pub(crate) fn total(&self) -> Option<NonZeroU64> {
        self.total
    }

    pub(crate) fn add(&mut self, change: usize) {
        self.free += change as u64;
        if self.availible() > 0 {
            self.can_write.store(true, Ordering::Release);
            self.write_notify.notify_one()
        }
    }

    pub(crate) fn remove(&mut self, change: usize) {
        self.free = self.free.saturating_sub(change as u64);
        if self.availible() == 0 {
            self.can_write.store(false, Ordering::Release);
        }
    }
}

pub(crate) fn new(capacity: Option<NonZeroU64>) -> (CapacityWatcher, Capacity) {
    let notify = Arc::new(Notify::new());
    let can_write = Arc::new(AtomicBool::new(true));
    (
        CapacityWatcher {
            notify: notify.clone(),
            can_write: can_write.clone(),
        },
        match capacity {
            Some(bytes) => Capacity {
                total: Some(bytes),
                free: bytes.get(),
                write_notify: notify,
                can_write,
            },
            None => Capacity {
                total: None,
                free: u64::MAX,
                write_notify: notify,
                can_write,
            },
        },
    )
}
