use std::net::IpAddr;

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
pub struct Bandwidth(usize);

impl Bandwidth {
    pub fn bytes(n: usize) -> Self {
        Self(n)
    }
}
