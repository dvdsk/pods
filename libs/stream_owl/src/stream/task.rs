use tokio::sync::mpsc;

use crate::http_client::StreamingClient;
use crate::network::Network;

use super::Error;

pub struct Canceld;

pub(crate) async fn new(
    url: http::Uri,
    mut seek_rx: mpsc::Receiver<u64>,
    restriction: Option<Network>,
) -> Result<Canceld, Error> {
    let mut client = StreamingClient::new(url, restriction).await?;
    let Some(_pos) = seek_rx.recv().await else {
        return Ok(Canceld);
    };

    let mut buffer = Vec::new();
    loop {
        match client {
            StreamingClient::Partial(client_with_stream) => {
                let mut reader = client_with_stream.into_reader();
                reader.read(&mut buffer, Some(1024)).await.unwrap();
                client = reader
                    .into_client()
                    .try_get_range(buffer.len() as u64, 1024)
                    .await
                    .unwrap();
            }
            StreamingClient::All(client_with_stream) => {
                let mut reader = client_with_stream.into_reader();
                reader.read(buffer, None).await.unwrap();
                break;
            }
        }
    }

    todo!()
}
