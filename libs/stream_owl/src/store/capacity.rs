use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Capacity {
    total: Arc<AtomicU64>,
}

impl Capacity {
    /// blocks till capacity is availible
    /// does not synchronize so only works
    /// with sequential access
    pub(crate) async fn wait_for_space(&self) {
        todo!()
    }

    pub(crate) fn availible(&self) -> usize {
        todo!()
    }

    pub(crate) fn set_total(&self, capacity: Option<NonZeroU64>) {
        let encoded = match capacity {
            Some(bytes) => bytes.get(),
            None => 0u64,
        };
        self.total.store(encoded, Ordering::Release)
    }

    pub(crate) fn total(&self) -> Option<NonZeroU64> {
        let encoded = self.total.load(Ordering::Acquire);
        NonZeroU64::new(encoded) // returns None if zero
    }

    pub(crate) fn add(&self, capacity: usize) {
        todo!()
    }

    pub(crate) fn remove(&self, capacity: usize) {
        todo!()
    }

    pub(crate) fn new() -> Self {
        todo!()
    }
}
