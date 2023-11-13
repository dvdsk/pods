use std::ops::Range;

#[derive(Debug, Clone)]
pub(super) struct Receiver {}

#[derive(Debug)]
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
    fn send(&self) -> Range<u64> {
        todo!()
    }
}
