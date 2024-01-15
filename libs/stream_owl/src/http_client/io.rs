//! Tokio IO integration for hyper
//! example code taken from hyper-util and adjusted to include
//! download bandwidth throttling
use std::io;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::AtomicUsize;
use std::sync::{Mutex, TryLockError};
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
use tokio::net::TcpStream;
use tracing::{debug, instrument, trace};

use crate::network::{BandwidthAllowed, BandwidthLim, BandwidthRx};
use crate::Bandwidth;

mod tcpstream_ext;
use tcpstream_ext::TcpStreamExt;

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
    pub struct ThrottlableIo {
        #[pin]
        inner: TcpStream,
        limiter: Option<RateLimiter<NotKeyed, InMemoryState, MonotonicClock, NoOpMiddleware<Instant>>>,
        sleeping: Option<Pin<Box<tokio::time::Sleep>>>,
        // limiter still needs to accept these
        still_pending_bytes: u32,
        new_bandwidth_lim: Arc<Mutex<BandwidthRx>>,
        os_socket_buf_size: usize,
    }
}

impl ThrottlableIo {
    /// Wrap a type implementing Tokio's IO traits.
    pub fn new(inner: TcpStream, bandwidth_lim: &BandwidthLim) -> Result<Self, io::Error> {
        Ok(Self {
            os_socket_buf_size: inner.send_buf_size()?,
            inner,
            limiter: None,
            still_pending_bytes: 0,
            sleeping: None,
            new_bandwidth_lim: bandwidth_lim.rx.clone(),
        })
    }

    unsafe fn perform_read(
        mut self: Pin<&mut Self>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<usize, std::io::Error>> {
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

impl ThrottlableIo {
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

    #[instrument(level = "debug", skip(self))]
    fn remove_lim(self: &mut Pin<&mut Self>) -> Result<(), std::io::Error> {
        self.set_lim(Bandwidth::practically_infinite())?;
        let this = self.as_mut().project();
        // drop the sleep so we resume immediately
        *this.sleeping = None;
        *this.limiter = None;
        debug!("rate limiter removed");
        Ok(())
    }

    #[instrument(level = "debug", skip(self))]
    fn set_lim(self: &mut Pin<&mut Self>, limit: Bandwidth) -> Result<(), std::io::Error> {
        let this = self.as_mut().project();
        let send_buf_size = limit.optimal_send_buf_size(*this.os_socket_buf_size);
        this.inner.set_send_buf_size(send_buf_size)?;

        let burst_size = limit.optimal_burst_size(send_buf_size);
        let limit = NonZeroU32::new(limit.0.get()).unwrap();
        let quota = Quota::per_second(limit).allow_burst(burst_size);
        debug!("New ratelimiter quota: {quota:?}, socket send_buf size is: {send_buf_size}");
        let new = RateLimiter::direct_with_clock(quota, &MonotonicClock::default());
        *this.limiter = Some(new);
        Ok(())
    }

    fn handle_bandwidth_lim_changes(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Result<(), std::io::Error> {
        let new_bandwidth_lim = {
            let this = self.as_mut().project();

            let mut new_bandwidth_lim = match this.new_bandwidth_lim.try_lock() {
                Ok(guard) => guard,
                Err(TryLockError::WouldBlock) => return Ok(()),
                Err(err @ TryLockError::Poisoned(_)) => panic!("{err}"),
            };
            new_bandwidth_lim.poll_recv(cx)
        };
        match new_bandwidth_lim {
            Poll::Ready(None) => {
                debug!("Stream handle dropped");
                return Ok(())
            }
            Poll::Ready(Some(BandwidthAllowed::Limited(limit))) => self.set_lim(limit)?,
            Poll::Ready(Some(BandwidthAllowed::UnLimited)) => self.remove_lim()?,
            Poll::Pending => (),
        }
        Ok(())
    }

    fn update_limiter(mut self: Pin<&mut Self>, n: usize, cx: &mut Context<'_>) {
        let Some(limiter) = self.limiter.as_mut() else {
            return;
        };

        let Some(n) = NonZeroU32::new(n as u32) else {
            return;
        };

        if let Err(until) = limiter.check_n(n).expect(&format!(
            "There should always be enough capacity, needed: {n}"
        )) {
            let next_call_allowed = until.earliest_possible();
            self.still_pending_bytes = n.get();
            self.sleep(cx, next_call_allowed);
            let dur = next_call_allowed.duration_since(Instant::now());
            trace!("next read is let through in {dur:?}");
        }
    }

    fn recheck_limiter(self: &mut Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let Some(pending) = NonZeroU32::new(self.still_pending_bytes) else {
            return Poll::Ready(());
        };

        let Some(limiter) = self.limiter.as_mut() else {
            return Poll::Ready(());
        };

        if let Err(until) = limiter.check_n(pending).expect(&format!(
            "There should always be enough capacity, needed: {}",
            pending
        )) {
            let next_call_allowed = until.earliest_possible();
            self.as_mut().sleep(cx, next_call_allowed);
            let dur = next_call_allowed.duration_since(Instant::now());
            trace!("next read is let through in {dur:?}");
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }

    fn do_sleep(self: &mut Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let poll_res = {
            let Some(fut) = self.sleeping.as_mut() else {
                return Poll::Ready(());
            };

            Future::poll(fut.as_mut(), cx)
        };

        if poll_res.is_ready() {
            self.sleeping = None;
        }
        poll_res
    }
}

impl hyper::rt::Read for ThrottlableIo {
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

        if let Poll::Pending = self.recheck_limiter(cx) {
            return Poll::Pending;
        }

        let Poll::Ready(read_res) = (unsafe { self.as_mut().perform_read(buf, cx) }) else {
            return Poll::Pending;
        };

        let n_read = match read_res {
            Err(e) => return Poll::Ready(Err(e)),
            Ok(n) => n,
        };
        // TODO: remove/replace with bandwidth tracking <13-01-24, dvdsk> 
        static TOTAL: AtomicUsize = AtomicUsize::new(0);
        let prev = TOTAL.fetch_add(n_read, Relaxed);
        trace!("total fetched: {}", prev + n_read);

        self.update_limiter(n_read, cx);

        Poll::Ready(Ok(()))
    }
}

impl hyper::rt::Write for ThrottlableIo {
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
