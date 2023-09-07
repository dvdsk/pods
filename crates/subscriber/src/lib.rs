mod sub;
pub use sub::{Sub, Clients, ClientsMap, Senders};

mod reader;
pub use reader::Reader;

use traits::{DataUpdate, Registration};

pub trait Subs: Send + Sync + std::fmt::Debug {
    fn senders(&self) -> &sub::Senders;
}

pub trait Needed<C, S>:  std::fmt::Debug  where S: Subs {
    fn update(&self, data: &C) -> DataUpdate;
    fn subs(&self, subs: &S) -> Vec<Registration>;
}
