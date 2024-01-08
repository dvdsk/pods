//! Tokio IO integration for hyper
//! example code taken from hyper-util and adjusted to include
//! download bandwidth throttling
use std::mem;
use std::sync::Mutex;
use std::{
    future::Future,
    num::NonZeroU32,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};

use governor::{
    clock::MonotonicClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use pin_project_lite::pin_project;
use tokio::sync::mpsc;

use crate::Bandwidth;

#[derive(Debug)]
pub(crate) enum BandwidthAllowed {
    Limited(Bandwidth),
    UnLimited,
}

impl BandwidthAllowed {
    fn bytes_per_second(&self) -> NonZeroU32 {
        match self {
            BandwidthAllowed::Limited(b) => b.bytes_per_second(),
            BandwidthAllowed::UnLimited => NonZeroU32::MAX,
        }
    }
}

type BandwidthRx = mpsc::Receiver<BandwidthAllowed>;
pub(crate) type BandwidthTx = mpsc::Sender<BandwidthAllowed>;

pin_project! {
    /// A wrapper adding rate limiting for a type that
    /// implements Tokio's IO traits while implementing
    /// the hyper::rt io traits.
    ///
    /// Note:
    /// This is aimed at clients downloading. Rate limiting is
    /// only applied to the read side.
    ///
    /// The rate limiting is always slightly behind as it does not
    /// limit the amount of bytes that _can_ be received but acts after
    /// the OS read call. There are quite some buffers, not only
    /// on the system this is running on, between that read call and
    /// the transmitter.
    ///
    /// After a (short) while the rate limit will effectively be
    /// used by the transmitter.
    #[derive(Debug)]
    pub struct ThrottlableIo<T> {
        #[pin]
        inner: T,
        limiter: RateLimiter<NotKeyed, InMemoryState, MonotonicClock, NoOpMiddleware<Instant>>,
        sleeping: Option<Pin<Box<tokio::time::Sleep>>>,
        new_bandwidth_lim: Arc<Mutex<BandwidthRx>>,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BandwidthLim {
    rx: Arc<Mutex<BandwidthRx>>,
}

impl BandwidthLim {
    pub(crate) fn from_init(init: BandwidthAllowed) -> (Self, BandwidthTx) {
        let (tx, rx) = mpsc::channel(12);
        let rx = Arc::new(Mutex::new(rx));
        if let BandwidthAllowed::Limited(_) = init {
            tx.try_send(init)
                .expect("First send can not fail on non zero cap channel");
        }
        (Self { rx }, tx)
    }

    fn init_quota(&self) -> Quota {
        let init = self
            .rx
            .try_lock()
            .expect("This conn is ended before new is started")
            .try_recv()
            .unwrap_or(BandwidthAllowed::UnLimited);
        let quota = init.bytes_per_second();
        Quota::per_second(quota)
    }
}

impl<T> ThrottlableIo<T> {
    /// Wrap a type implementing Tokio's IO traits.
    pub fn new(inner: T, bandwidth_lim: &BandwidthLim) -> Self {
        Self {
            inner,
            limiter: RateLimiter::direct_with_clock(
                bandwidth_lim.init_quota(),
                &MonotonicClock::default(),
            ),
            sleeping: None,
            new_bandwidth_lim: bandwidth_lim.rx.clone(),
        }
    }

    unsafe fn perform_read(
        mut self: Pin<&mut Self>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<usize, std::io::Error>>
    where
        T: tokio::io::AsyncRead,
    {
        let this = self.as_mut().project();
        let mut tbuf = tokio::io::ReadBuf::uninit(buf.as_mut());
        let n = match tokio::io::AsyncRead::poll_read(this.inner, cx, &mut tbuf) {
            Poll::Ready(Ok(())) => tbuf.filled().len(),
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => return Poll::Pending,
        };
        buf.advance(n);
        Poll::Ready(Ok(n))
    }
}

fn handle_gone_err() -> std::io::Error {
    todo!()
}

impl<T> ThrottlableIo<T> {
    fn sleep(self: Pin<&mut Self>, cx: &mut Context<'_>, next_call_allowed: Instant) {
        let this = self.project();
        let fut = tokio::time::sleep_until(next_call_allowed.into());
        let mut fut = Box::pin(fut);
        // poll once to register sleep with context
        match Future::poll(fut.as_mut(), cx) {
            Poll::Ready(()) => (),
            Poll::Pending => *this.sleeping = Some(fut),
        };
    }

    fn remove_lim(self: &mut Pin<&mut Self>) {
        self.set_lim(Bandwidth::practically_infinite());
        // now drop the sleep so we resume immediately
        let this = self.as_mut().project();
        *this.sleeping = None;
    }

    fn set_lim(self: &mut Pin<&mut Self>, limit: Bandwidth) {
        let this = self.as_mut().project();
        let new_quota = Quota::per_second(limit.bytes_per_second());
        let mut new = RateLimiter::direct_with_clock(new_quota, &MonotonicClock::default());
        mem::swap(this.limiter, &mut new);
    }

    fn handle_bandwidth_lim_changes(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Result<(), std::io::Error> {
        let new_bandwidth_lim = {
            let this = self.as_mut().project();

            let mut new_bandwidth_lim = this
                .new_bandwidth_lim
                .try_lock()
                .expect("Only one Io conn per bandwidth_lim_rx");
            new_bandwidth_lim.poll_recv(cx)
        };
        match new_bandwidth_lim {
            Poll::Ready(None) => return Err(handle_gone_err()),
            Poll::Ready(Some(BandwidthAllowed::Limited(limit))) => self.set_lim(limit),
            Poll::Ready(Some(BandwidthAllowed::UnLimited)) => self.remove_lim(),
            Poll::Pending => (),
        }
        Ok(())
    }

    fn update_limiter(self: Pin<&mut Self>, n: usize, cx: &mut Context<'_>) {
        if let Some(n) = NonZeroU32::new(n as u32) {
            if let Err(until) = self
                .limiter
                .check_n(n)
                .expect("There should always be enough capacity")
            {
                let next_call_allowed = until.earliest_possible();
                self.sleep(cx, next_call_allowed);
            }
        }
    }

    fn do_sleep(self: &mut Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.as_mut().project();
        if let Some(fut) = this.sleeping.as_mut() {
            Future::poll(fut.as_mut(), cx)
        } else {
            Poll::Ready(())
        }
    }
}

impl<T> hyper::rt::Read for ThrottlableIo<T>
where
    T: tokio::io::AsyncRead,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        if let Err(e) = self.as_mut().handle_bandwidth_lim_changes(cx) {
            return Poll::Ready(Err(e));
        }

        if let Poll::Pending = self.do_sleep(cx) {
            return Poll::Pending;
        }

        let Poll::Ready(read_res) = (unsafe { self.as_mut().perform_read(buf, cx) }) else {
            return Poll::Pending;
        };

        let n_read = match read_res {
            Err(e) => return Poll::Ready(Err(e)),
            Ok(n) => n,
        };

        self.update_limiter(n_read, cx);

        Poll::Ready(Ok(()))
    }
}

impl<T> hyper::rt::Write for ThrottlableIo<T>
where
    T: tokio::io::AsyncWrite,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        tokio::io::AsyncWrite::poll_write(self.project().inner, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        tokio::io::AsyncWrite::poll_flush(self.project().inner, cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        tokio::io::AsyncWrite::poll_shutdown(self.project().inner, cx)
    }

    fn is_write_vectored(&self) -> bool {
        tokio::io::AsyncWrite::is_write_vectored(&self.inner)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        tokio::io::AsyncWrite::poll_write_vectored(self.project().inner, cx, bufs)
    }
}

impl<T> tokio::io::AsyncRead for ThrottlableIo<T>
where
    T: hyper::rt::Read,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        tbuf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let filled = tbuf.filled().len();
        let sub_filled = unsafe {
            let mut buf = hyper::rt::ReadBuf::uninit(tbuf.unfilled_mut());

            match hyper::rt::Read::poll_read(self.project().inner, cx, buf.unfilled()) {
                Poll::Ready(Ok(())) => buf.filled().len(),
                other => return other,
            }
        };

        let n_filled = filled + sub_filled;
        // At least sub_filled bytes had to have been initialized.
        let n_init = sub_filled;
        unsafe {
            tbuf.assume_init(n_init);
            tbuf.set_filled(n_filled);
        }

        Poll::Ready(Ok(()))
    }
}

impl<T> tokio::io::AsyncWrite for ThrottlableIo<T>
where
    T: hyper::rt::Write,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        hyper::rt::Write::poll_write(self.project().inner, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        hyper::rt::Write::poll_flush(self.project().inner, cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        hyper::rt::Write::poll_shutdown(self.project().inner, cx)
    }

    fn is_write_vectored(&self) -> bool {
        hyper::rt::Write::is_write_vectored(&self.inner)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        hyper::rt::Write::poll_write_vectored(self.project().inner, cx, bufs)
    }
}
