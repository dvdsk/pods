mod manager;
mod network;
mod reader;
mod store;

/// Glue between store and stream/http_client
mod target;
mod stream;
pub mod http_client;

#[macro_use]
mod vecdeque;

/// internal use only! in time move this to tests/common/common.rs
/// for now RA needs it here and we need RA
pub mod testing;

pub use stream::StreamBuilder;

pub use manager::Error as ManagerError;
pub use manager::Manager;
pub use stream::StreamDone as StreamDone;
pub use stream::Error as StreamError;
pub use stream::Id as StreamId;
pub use stream::Handle as StreamHandle;
pub use stream::ManagedHandle as ManagedStreamHandle;
pub use reader::Reader;
pub use network::{list_interfaces, Bandwidth};
