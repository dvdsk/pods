/// Interface for reading streaming/downloading data
use async_trait::async_trait;

mod http_client;
mod manager;
mod network;
mod reader;
mod store;
mod stream;

#[async_trait]
trait Appender {
    // returns amount written, does not guarentee entire buffer is written
    async fn append(&mut self, buf: &[u8]) -> Result<usize, std::io::Error>;
}

pub use manager::Error as ManagerError;
pub use manager::Manager;
pub use stream::Error as StreamError;
pub use stream::Id as StreamId;

pub use network::list_interfaces;
