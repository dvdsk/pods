/// Tracks the size of data send via the http stream.
///
/// No benchmarks motivating these optimizations, they are just for fun.
///
/// This is an overly optimized enum that tracks the size of the date send via the http_stream. The
/// only reason it is not an Arc<Mutex<Enum>> is because I wanted to write it like this for fun.
/// Feel free to replace/refactor.
use http::{self, header, HeaderValue, StatusCode};
use hyper::Response;
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
    requests_analyzed: AtomicUsize,
}

#[derive(Clone)]
pub(crate) struct Size(Arc<SizeInner>);

#[derive(Debug, PartialEq)]
enum SizeVariant {
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
        tracing::debug!("set stream size to: {var:?}");
        let new = var.encode();
        let previous = self.0.value.swap(new, Ordering::Release);
        if previous != new {
            self.0.notify.notify_waiters();
        }
    }

    fn get(&self) -> SizeVariant {
        SizeVariant::decode(self.0.value.load(Ordering::Acquire))
    }

    pub(crate) fn mark_stream_end(&self, pos: u64) {
        self.set(SizeVariant::StreamEnded(pos));
    }

    #[tracing::instrument(level = "debug", skip(response), 
      fields(headers = ?response.headers(), status = %response.status()))]
    pub(crate) fn update_from_headers<T>(&mut self, response: &Response<T>) {
        if response.status() != StatusCode::FOUND {
            self.0.requests_analyzed.fetch_add(1, Ordering::Relaxed);
        }
        let headers = response.headers();

        match response.status() {
            StatusCode::OK => {
                if let Some(content_length) = headers
                    .get(header::CONTENT_LENGTH)
                    .map(HeaderValue::to_str)
                    .and_then(Result::ok)
                    .map(|len| u64::from_str_radix(len, 10))
                    .and_then(Result::ok)
                {
                    self.set(SizeVariant::Known(content_length))
                }
            }
            StatusCode::PARTIAL_CONTENT | StatusCode::RANGE_NOT_SATISFIABLE => {
                if let Some(range_total) = headers
                    .get(header::CONTENT_RANGE)
                    .map(HeaderValue::to_str)
                    .and_then(Result::ok)
                    .filter(|range| range.starts_with("bytes"))
                    .and_then(|range| range.rsplit_once("/"))
                    .map(|(_, total)| u64::from_str_radix(total, 10))
                    .and_then(Result::ok)
                {
                    self.set(SizeVariant::Known(range_total))
                }
            }
            _ => self.set(SizeVariant::Unknown),
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

    #[test]
    fn analyze_headers() {
        fn test_response<'a>(key: &'static str, val: &'a str) -> Response<&'a str> {
            let mut input = Response::new("");
            *input.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
            let headers = input.headers_mut();
            headers.insert(key, val.try_into().unwrap());
            input
        }

        let mut size = Size::default();
        size.update_from_headers(&test_response("content-range", "bytes */10000"));
        assert_eq!(size.get(), SizeVariant::Known(10_000));
    }
}
