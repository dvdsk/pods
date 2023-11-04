use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::task;

use tokio::io::AsyncWrite;

#[derive(Debug, Clone)]
pub(crate) struct Counter(Arc<AtomicU64>);

impl Counter {
    pub(crate) fn written(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }

    fn add(&self, n: u64) {
        self.0.fetch_add(n, Ordering::Release);
    }
}

pub(crate) struct CountingWriter<T> {
    pub(crate) inner: T,
    pub(crate) counter: Counter,
}

impl<T: Unpin> CountingWriter<T> {
    pub(crate) fn new(writer: T) -> CountingWriter<T> {
        Self {
            inner: writer,
            counter: Counter(Arc::new(AtomicU64::new(0))),
        }
    }

    pub(crate) fn inner_pinned(self: Pin<&mut Self>) -> Pin<&mut T> {
        let unpinned = Pin::into_inner(self);
        let inner_pinned = Pin::new(&mut unpinned.inner);
        inner_pinned
    }

    pub(crate) fn written(&self) -> u64 {
        self.counter.written()
    }

    pub(crate) fn counter(&self) -> Counter {
        self.counter.clone()
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for CountingWriter<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> task::Poll<Result<usize, std::io::Error>> {
        let unpinned = Pin::into_inner(self);
        let inner = Pin::new(&mut unpinned.inner);
        let res = inner.poll_write(cx, buf);
        if let task::Poll::Ready(Ok(n)) = res {
            unpinned.counter.add(n as u64);
        }
        res
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Result<(), std::io::Error>> {
        self.inner_pinned().poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Result<(), std::io::Error>> {
        self.inner_pinned().poll_shutdown(cx)
    }
}
