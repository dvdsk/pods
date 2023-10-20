/// Interface for reading streaming/downloading data

mod reader;
mod storage;
mod stream;
mod manager;
mod network;
mod http_client;

pub use manager::Error as ManagerError;
pub use stream::Error as StreamError;
pub use stream::Id as StreamId;
pub use manager::Manager;

