use std::ops::Range;

#[derive(Debug, Clone)]
pub(super) struct Receiver {}

#[derive(Debug, Clone)]
pub(super) struct Sender {}

// initial range is 0..0
pub(super) fn channel() -> (Sender, Receiver) {
    todo!()
}

impl Receiver {
    pub(super) fn blocking_wait_for(&self, needed: u64) {
        todo!()
    }
}

impl Sender {
    pub(super) fn send(&self, range: Range<u64>) {
        todo!()
    }
}
