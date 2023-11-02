use std::task;
use std::sync::atomic::Ordering;
use std::pin::Pin;
use std::sync::atomic::AtomicUsize;

use tokio::io::AsyncWrite;

pub(crate) struct CountingWriter<T> {
    pub(crate) inner: T,
    pub(crate) written: AtomicUsize,
}

impl<T: Unpin> CountingWriter<T> {
    pub(crate) fn new(writer: T) -> CountingWriter<T> {
        Self {
            inner: writer,
            written: AtomicUsize::new(0),
        }
    }

    pub(crate) fn inner_pinned(self: Pin<&mut Self>) -> Pin<&mut T> {
        let unpinned = Pin::into_inner(self);
        let inner_pinned = Pin::new(&mut unpinned.inner);
        inner_pinned
    }

    pub(crate) fn written(&self) -> usize {
        self.written.load(Ordering::Acquire)
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
            unpinned.written.fetch_add(n, Ordering::Release);
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

