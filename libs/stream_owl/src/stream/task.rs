use tokio::sync::mpsc;

use crate::http_client::Client;
use crate::network::Network;

use super::Error;

pub struct Canceld;

pub(crate) async fn new(url: http::Uri, mut seek_rx: mpsc::Receiver<u64>, restriction: Option<Network>) -> Result<Canceld, Error> {
    let client = Client::new(url, restriction).await?;
    let Some(pos) = seek_rx.recv().await else {
        return Ok(Canceld)
    }; 

    loop {
        let get_data = client.get_range(pos, pos+4_096);
        let new_pos = seek_rx.recv();

    }
}
