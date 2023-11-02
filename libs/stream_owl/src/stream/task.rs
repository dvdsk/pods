use futures::FutureExt;
use tokio::sync::mpsc;

use crate::http_client::Error as HttpError;
use crate::http_client::StreamingClient;
use crate::network::Network;
use crate::stream::task::counting_writer::CountingWriter;

use super::Error;
use futures_concurrency::future::Race;

pub struct Canceld;

mod counting_writer;

enum Res1 {
    Seek(Option<u64>),
    NewClient(Result<StreamingClient, HttpError>),
}

// the writer will limit how fast we recieve data using backpressure
pub(crate) async fn new(
    url: http::Uri,
    mut seek_rx: mpsc::Receiver<u64>,
    restriction: Option<Network>,
) -> Result<Canceld, Error> {
    let mut start_pos = 0;
    let chunk_size = 10_000;

    let mut client = loop {
        let recieve_seek = seek_rx.recv().map(Res1::Seek);
        let build_client =
            StreamingClient::new(url.clone(), restriction.clone(), start_pos, chunk_size)
                .map(Res1::NewClient);

        match (build_client, recieve_seek).race().await {
            Res1::NewClient(client) => break client?,
            Res1::Seek(None) => return Ok(Canceld),
            Res1::Seek(Some(pos)) => start_pos = pos,
        }
    };

    let buffer = Vec::new();
    let mut writer = CountingWriter::new(buffer);
    loop {
        match client {
            StreamingClient::Partial(client_with_stream) => {
                client = handle_partial_stream(client_with_stream, &mut writer, chunk_size).await?;
            }
            StreamingClient::All(client_with_stream) => {
                let mut reader = client_with_stream.into_reader();
                reader.read_to_writer(&mut writer, None).await.unwrap();
                break;
            }
        }
    }

    todo!()
}

enum Res2 {
    Seek(Option<u64>),
    Write(Result<(), HttpError>),
}

enum Res3 {
    Seek(Option<u64>),
    GetRange(Result<StreamingClient, HttpError>),
}

async fn handle_partial_stream(
    client_with_stream: crate::http_client::ClientStreamingPartial,
    writer: &mut CountingWriter<Vec<u8>>,
    chunk_size: u64,
    seek_rx: &mut mpsc::Receiver<u64>,
) -> Result<StreamingClient, Error> {
    let mut next_pos = None;

    let mut reader = client_with_stream.into_reader();
    let read_to_writer = reader
        .read_to_writer(writer, Some(chunk_size as usize))
        .map(Res2::Write);
    let got_seek = seek_rx.recv().map(Res2::Seek);
    match (read_to_writer, got_seek).race().await {
        Res2::Seek(None) => return todo!(),
        Res2::Seek(Some(pos)) => next_pos = Some(pos),
        Res2::Write(res) => res?,
    }

    // we do not race this with seek_rx as:
    // it takes only a short wile
    let mut client = reader
        .try_into_client()
        .expect("should not read less then we requested");
    let res = client
        .try_get_range(writer.written() as u64, chunk_size)
        .await;

    match res {
        Ok(_) => ,
        Err(HttpError::InvalidRange) => todo!(),
        Err(e) => return Err(Error::HttpClient(e)),
    }
}
