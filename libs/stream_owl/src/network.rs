use std::io;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::Mutex;

use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use socket2::Domain;
use socket2::Socket;
use socket2::Type;
use tokio::sync::mpsc;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct Network {
    pub(crate) name: String,
    pub(crate) addr: IpAddr,
}

impl Network {
    pub(crate) fn addr(&self) -> IpAddr {
        self.addr
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub fn list_interfaces() -> Result<Vec<Network>, network_interface::Error> {
    NetworkInterface::show().map(|n| {
        n.into_iter()
            .filter(|n| !n.addr.is_empty())
            .map(|n| Network {
                name: n.name,
                addr: n
                    .addr
                    .first()
                    .expect("filter guarantees this is not empty")
                    .ip(),
            })
            .collect()
    })
}

// internally uses bytes per second
#[derive(Debug, Clone, Copy)]
pub struct Bandwidth(pub(crate) NonZeroU32);

#[derive(Debug, thiserror::Error)]
pub enum BandwidthError {
    #[error("On this system the bandwidth limit must be bigger {minimum} bytes")]
    TooSmall { minimum: usize },
    #[error("Checking minimal socket buffer size")]
    CheckingSocketOpt(io::Error),
}

fn minimal_recv_buf_size() -> Result<usize, io::Error> {
    let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;
    socket.set_recv_buffer_size(1)?;
    socket.recv_buffer_size()
}

impl Bandwidth {
    pub fn kbytes(n: u32) -> Result<Self, BandwidthError> {
        Self::bytes(n * 1000)
    }

    /// Bandwidth in bits per second
    #[instrument(err)]
    pub fn bytes(n: u32) -> Result<Self, BandwidthError> {
        let minimum = minimal_recv_buf_size().map_err(BandwidthError::CheckingSocketOpt)?;
        if (n as usize) < minimum {
            return Err(BandwidthError::TooSmall { minimum });
        }
        Ok(Self(
            NonZeroU32::new(n).expect("socket buf size should be bigger then zero"),
        ))
    }

    pub fn practically_infinite() -> Self {
        Self(NonZeroU32::MAX)
    }

    /// use os_default as max
    #[instrument(level = "debug", ret)]
    pub(crate) fn optimal_send_buf_size(&self, os_default: usize) -> usize {
        (self.0.get() as usize).min(os_default)
    }

    #[instrument(level = "debug", ret)]
    pub(crate) fn optimal_burst_size(&self, send_buf_size: usize) -> NonZeroU32 {
        self.0
            .saturating_mul(NonZeroU32::new(2).unwrap())
            .max(NonZeroU32::new(send_buf_size as u32).unwrap())
    }
}

pub(crate) type BandwidthRx = mpsc::Receiver<BandwidthAllowed>;
pub(crate) type BandwidthTx = mpsc::Sender<BandwidthAllowed>;

#[derive(Debug)]
pub(crate) enum BandwidthAllowed {
    Limited(Bandwidth),
    UnLimited,
}

#[derive(Debug, Clone)]
pub(crate) struct BandwidthLim {
    pub(crate) rx: Arc<Mutex<BandwidthRx>>,
}

impl BandwidthLim {
    pub(crate) fn new(init: BandwidthAllowed) -> (Self, BandwidthTx) {
        let (tx, rx) = mpsc::channel(12);
        let rx = Arc::new(Mutex::new(rx));
        if let BandwidthAllowed::Limited(_) = init {
            tx.try_send(init)
                .expect("First send can not fail on non zero cap channel");
        }
        (Self { rx }, tx)
    }
}
