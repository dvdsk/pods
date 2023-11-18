use std::num::NonZeroU64;
use std::sync::Arc;

use tokio::sync::Notify;

#[derive(Debug)]
pub struct Capacity {
    total: Option<NonZeroU64>,
    free: u64,
    availible: Arc<Notify>,
}

#[derive(Debug, Clone)]
pub struct CapacityWatcher {
    availible: Arc<Notify>,
}

impl CapacityWatcher {
    /// blocks till capacity is availible
    /// does not synchronize so only works
    /// with sequential access
    pub(crate) async fn wait_for_space(&self) {
        self.availible.notified().await;
    }
}

impl Capacity {
    pub(crate) fn availible(&self) -> usize {
        // cast to smaller unsigned saturates at upper bound
        match self.total {
            Some(limited) => (limited.get() - self.free) as usize,
            None => usize::MAX,
        }
    }

    pub(crate) fn total(&self) -> Option<NonZeroU64> {
        self.total
    }

    pub(crate) fn add(&mut self, change: usize) {
        self.free += change as u64;
        if self.availible() > 0 {
            self.availible.notify_waiters()
        }
    }

    pub(crate) fn remove(&mut self, change: usize) {
        self.free = self.free.saturating_sub(change as u64);
    }

    fn new(capacity: Option<NonZeroU64>, availible: Arc<Notify>) -> Self {
        match capacity {
            Some(bytes) => Self {
                total: Some(bytes),
                free: bytes.get(),
                availible,
            },
            None => Self {
                total: None,
                free: u64::MAX,
                availible,
            },
        }
    }
}

pub(crate) fn new(capacity: Option<NonZeroU64>) -> (CapacityWatcher, Capacity) {
    let notify = Arc::new(Notify::new());
    (
        CapacityWatcher {
            availible: notify.clone(),
        },
        Capacity::new(capacity, notify),
    )
}
