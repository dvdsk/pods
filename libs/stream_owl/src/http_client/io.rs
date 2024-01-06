//! Tokio IO integration for hyper
//! example code taken from hyper-util and adjusted to include
//! download bandwidth throttling
use std::{
    future::Future,
    num::NonZeroU32,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use governor::{
    clock::MonotonicClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use pin_project_lite::pin_project;
use tokio::sync::mpsc::Receiver;

use crate::Bandwidth;

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
        config_changes: Receiver<ConfigChange>,
        config: Config,
    }
}

#[derive(Debug)]
struct Config {
    paused: bool,
    bandwidth_limit: Option<Bandwidth>,
}

#[derive(Debug)]
enum ConfigChange {
    Paused,
    Resumed,
    BandwidthLimitSet,
    BandwidthLimitUpdated,
    BandwidthLimitRemoved,
}

impl Config {
    fn quota(&self) -> Quota {
        let quota = self
            .bandwidth_limit
            .unwrap_or_else(Bandwidth::practically_infinite);
        Quota::per_second(quota.bytes_per_second())
    }
}

impl<T> ThrottlableIo<T> {
    /// Wrap a type implementing Tokio's IO traits.
    pub fn new(inner: T, config: Config, config_changes: Receiver<ConfigChange>) -> Self {
        Self {
            inner,
            limiter: RateLimiter::direct_with_clock(config.quota(), &MonotonicClock::default()),
            sleeping: None,
            config_changes,
            config,
        }
    }
}

impl<T> ThrottlableIo<T> {
    fn update_config(self: Pin<&mut Self>, cx: &mut Context<'_>, change: ConfigChange) {
        match change {
            ConfigChange::Paused => self.sleep(cx, forever()),
            ConfigChange::Resumed => self.cancel_sleep(),
            ConfigChange::BandwidthLimitSet => todo!(),
            ConfigChange::BandwidthLimitUpdated => todo!(),
            ConfigChange::BandwidthLimitRemoved => self.remove_bandwidth_limit(),
        }
    }

    fn sleep(self: Pin<&mut Self>, cx: &mut Context<'_>, dur: Duration) {
        let this = self.project();
        let fut = tokio::time::sleep(dur);
        let mut fut = Box::pin(fut);
        // poll once to register sleep with context
        match Future::poll(fut.as_mut(), cx) {
            Poll::Ready(()) => (),
            Poll::Pending => *this.sleeping = Some(fut),
        };
    }

    fn cancel_sleep(self: Pin<&mut Self>) {
        let this = self.project();
        *this.sleeping = None;
    }

    fn remove_bandwidth_limit(self: Pin<&mut Self>) {
        let this = self.project();
        this.limiter
    }
}

fn forever() -> Duration {
    Duration::MAX
}

fn handle_gone_err() -> std::io::Error {
    todo!()
}

impl<T> hyper::rt::Read for ThrottlableIo<T>
where
    T: tokio::io::AsyncRead,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let n = unsafe {
            let this = self.project();

            match this.config_changes.poll_recv(cx) {
                Poll::Ready(None) => return Poll::Ready(Err(handle_gone_err())),
                Poll::Ready(Some(change)) => self.update_config(cx, change),
                Poll::Pending => (),
            }

            if let Some(fut) = this.sleeping.as_mut() {
                match Future::poll(fut.as_mut(), cx) {
                    Poll::Ready(()) => (),
                    Poll::Pending => return Poll::Pending,
                };
            }

            let mut tbuf = tokio::io::ReadBuf::uninit(buf.as_mut());
            let n = match tokio::io::AsyncRead::poll_read(this.inner, cx, &mut tbuf) {
                Poll::Ready(Ok(())) => tbuf.filled().len(),
                other => return other,
            };

            if let Some(n) = NonZeroU32::new(n as u32) {
                if let Err(until) = this
                    .limiter
                    .check_n(n)
                    .expect("There should always be enough capacity")
                {
                    let next_call_allowed = until.earliest_possible();
                    let fut = tokio::time::sleep_until(next_call_allowed.into());
                    let mut fut = Box::pin(fut);
                    // poll once to register sleep with context
                    match Future::poll(fut.as_mut(), cx) {
                        Poll::Ready(()) => (),
                        Poll::Pending => *this.sleeping = Some(fut),
                    };
                }
            }

            n
        };

        unsafe {
            buf.advance(n);
        }
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
