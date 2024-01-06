use std::net::IpAddr;
use std::num::NonZeroU32;

use network_interface::{NetworkInterface, NetworkInterfaceConfig};

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

#[derive(Debug, Clone, Copy)]
pub struct Bandwidth(NonZeroU32);
#[derive(Debug)]
pub struct IsZero;

impl Bandwidth {
    pub fn bytes(n: u32) -> Result<Self, IsZero> {
        Ok(Self(NonZeroU32::new(n).ok_or(IsZero)?))
    }

    pub fn bytes_per_second(&self) -> NonZeroU32 {
        self.0
    }

    pub fn practically_infinite() -> Self {
        Self(NonZeroU32::MAX)
    }
}
