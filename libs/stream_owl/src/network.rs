use std::net::IpAddr;
use std::num::NonZeroU32;

use network_interface::{NetworkInterface, NetworkInterfaceConfig};

#[derive(Debug, Clone)]
pub struct Network {
    pub(crate) name: String,
    pub(crate) interface_index: NonZeroU32,
    pub(crate) addr: IpAddr,
}

impl Network {
    pub(crate) fn addr(self) -> IpAddr {
        self.addr
    }
}

fn list_interfaces() -> Result<Vec<Network>, network_interface::Error> {
    NetworkInterface::show().map(|n| {
        n.into_iter()
            .filter(|n| !n.addr.is_empty())
            .map(|n| Network {
                name: n.name,
                addr: n
                    .addr
                    .first()
                    .expect("filter guarentees this is not empty")
                    .ip(),
                interface_index: NonZeroU32::new(n.index).unwrap(),
            })
            .collect()
    })
}

#[derive(Debug)]
pub struct Bandwith(usize);
