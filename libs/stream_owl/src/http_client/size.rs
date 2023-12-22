use super::ValidResponse;
/// Tracks the size of data send via the http stream.
///
/// No benchmarks motivating these optimizations, they are just for fun.
///
/// This is an overly optimized enum that tracks the size of the date send via the http_stream. The
/// only reason it is not an Arc<Mutex<Enum>> is because I wanted to write it like this for fun.
/// Feel free to replace/refactor.
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::Notify;

struct SizeInner {
    value: AtomicU64,
    /// need to check the current value has changed
    notify: Notify,
    // used to check whether the size might
    // become known soon
    //
    // - Redirects do not count
    // - Overflow is not realistic.
    pub requests_analyzed: AtomicUsize,
}

#[derive(Clone)]
pub(crate) struct Size(Arc<SizeInner>);

#[derive(Debug, PartialEq)]
pub(crate) enum SizeVariant {
    Unknown,
    Known(u64),
    StreamEnded(u64),
}

const STREAM_ENDED_MASK: u64 = 1 << 63;
const KNOWN_MASK: u64 = 1 << 62;

impl SizeVariant {
    fn encode(&self) -> u64 {
        match *self {
            Self::Unknown => u64::MAX,
            Self::Known(size) => {
                // should never encounter a size larger then 2^61
                // but if we do lower the value instead of changing the enum var
                size & !STREAM_ENDED_MASK | KNOWN_MASK
            }
            Self::StreamEnded(size) => {
                // should never encounter a size larger then 2^61
                // but if we do increase the value instead of changing the enum var
                size & !KNOWN_MASK | STREAM_ENDED_MASK
            }
        }
    }

    fn decode(val: u64) -> Self {
        if val == u64::MAX {
            Self::Unknown
        } else if val & STREAM_ENDED_MASK == STREAM_ENDED_MASK {
            Self::StreamEnded(val & !STREAM_ENDED_MASK)
        } else {
            Self::Known(val & !KNOWN_MASK)
        }
    }
}

impl core::fmt::Debug for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match SizeVariant::decode(self.0.value.load(Ordering::Relaxed)) {
            SizeVariant::Unknown => write!(f, "Size::Unknown"),
            SizeVariant::Known(bytes) => write!(f, "Size::Known({bytes})"),
            SizeVariant::StreamEnded(bytes) => write!(f, "Size::Unknown({bytes})"),
        }
    }
}

impl Default for Size {
    fn default() -> Self {
        Self(Arc::new(SizeInner {
            value: AtomicU64::new(SizeVariant::Unknown.encode()),
            notify: Notify::new(),
            requests_analyzed: AtomicUsize::new(0),
        }))
    }
}

pub(crate) struct Timeout;

impl Size {
    fn set(&self, var: SizeVariant) {
        let new = var.encode();
        let previous = self.0.value.swap(new, Ordering::Release);
        if previous != new {
            tracing::debug!(
                "stream size changed: {:?} -> {:?}",
                SizeVariant::decode(previous),
                var
            );
            self.0.notify.notify_waiters();
        }
    }

    fn get(&self) -> SizeVariant {
        SizeVariant::decode(self.0.value.load(Ordering::Acquire))
    }

    pub(crate) fn mark_stream_end(&self, pos: u64) {
        self.set(SizeVariant::StreamEnded(pos));
    }

    #[tracing::instrument(level = "debug", skip(response))]
    pub(crate) fn update(&mut self, response: &ValidResponse) {
        self.0.requests_analyzed.fetch_add(1, Ordering::Relaxed);
        if let Some(size) = response.stream_size() {
            self.set(SizeVariant::Known(size));
        } else {
            self.set(SizeVariant::Unknown);
        }
    }

    pub(crate) fn wait_for_known(
        &self,
        rt: &mut Runtime,
        timeout: Duration,
    ) -> Result<u64, Timeout> {
        let _guard = rt.enter();
        async fn get_size(size: &Size) -> u64 {
            size.0.notify.notified().await;
            size.known().unwrap()
        }
        let fut = tokio::time::timeout(timeout, get_size(self));
        rt.block_on(fut).map_err(|_| Timeout)
    }

    pub(crate) fn requests_analyzed(&self) -> usize {
        self.0.requests_analyzed.load(Ordering::Relaxed)
    }

    /// size can be unknown even for range requests (think live-stream)
    pub(crate) fn known(&self) -> Option<u64> {
        match self.get() {
            SizeVariant::Unknown => None,
            SizeVariant::Known(size) => Some(size),
            SizeVariant::StreamEnded(size) => Some(size),
        }
    }

    pub(crate) async fn eof_smaller_then(&self, pos: u64) {
        loop {
            let curr = self.get();
            match curr {
                SizeVariant::StreamEnded(eof) if eof < pos => break,
                _ => (),
            }
            self.0.notify.notified().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use super::*;

    #[test]
    fn encode_matches_decode() {
        let test_cases = [
            SizeVariant::Unknown,
            SizeVariant::Known(0),
            SizeVariant::Known(2u64.pow(61)),
            SizeVariant::StreamEnded(0),
            SizeVariant::StreamEnded(2u64.pow(61)),
        ];
        for val in test_cases {
            let encoded = val.encode();
            assert_eq!(SizeVariant::decode(encoded), val, "encoded: {encoded}");
        }

        let out_of_bound_cases = [
            (
                SizeVariant::Known(u64::MAX),
                SizeVariant::Known(2u64.pow(62) - 1),
            ),
            (
                SizeVariant::StreamEnded(u64::MAX),
                SizeVariant::StreamEnded(2u64.pow(62) - 1),
            ),
        ];
        for (broken_input, acceptable_output) in out_of_bound_cases {
            let encoded = broken_input.encode();
            assert_eq!(SizeVariant::decode(encoded), acceptable_output)
        }
    }
}
