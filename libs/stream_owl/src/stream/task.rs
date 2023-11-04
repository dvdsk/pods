use futures::FutureExt;
use tokio::io::AsyncWrite;
use tokio::sync::mpsc;

use crate::http_client::ClientStreamingAll;
use crate::http_client::Error as HttpError;
use crate::http_client::StreamingClient;
use crate::network::Network;
use crate::stream::task::counting_writer::CountingWriter;

use self::counting_writer::Counter;

use super::Error;
use futures_concurrency::future::Race;

pub struct Canceld;

mod counting_writer;

enum Res1 {
    Seek(Option<u64>),
    NewClient(Result<StreamingClient, HttpError>),
}

enum Res2 {
    Seek(Option<u64>),
    Write(Result<(), HttpError>),
}

// the writer will limit how fast we recieve data using backpressure
pub(crate) async fn new(
    url: http::Uri,
    storage: impl AsyncWrite + Unpin,
    mut seek_rx: mpsc::Receiver<u64>,
    restriction: Option<Network>,
) -> Result<Canceld, Error> {
    let mut start_pos = 0;
    let chunk_size = 10_000;
    let mut writer = CountingWriter::new(storage);

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

    loop {
        let stream_all_client = match client {
            StreamingClient::Partial(client_with_stream) => {
                let res =
                    stream_partial(client_with_stream, &mut writer, chunk_size, &mut seek_rx).await;
                match res {
                    StreamPartialRes::Canceld => return Ok(Canceld),
                    StreamPartialRes::Err(e) => return Err(e),
                    StreamPartialRes::StreamAllclient(client) => client,
                }
            }
            StreamingClient::All(client_with_stream) => client_with_stream,
        };

        let builder = stream_all_client.builder();
        let mut reader = stream_all_client.into_reader();
        let recieve_seek = recieve_actionable_seek(&mut seek_rx, writer.counter()).map(Res2::Seek);
        let write = reader.read_to_writer(&mut writer, None).map(Res2::Write);

        let pos = match (write, recieve_seek).race().await {
            Res2::Seek(Some(relevant_pos)) => relevant_pos,
            Res2::Seek(None) => return Ok(Canceld),
            Res2::Write(Err(e)) => return Err(Error::HttpClient(e)),
            Res2::Write(Ok(())) => match seek_rx.recv().await {
                Some(pos) => pos,
                None => return Ok(Canceld),
            },
        };

        // do not concurrently wait for seeks, since we probably wont
        // get range support so any seek would translate to starting the stream
        // again. Which is wat we are doing here.
        client = builder.connect(pos, chunk_size).await?;
    }
}

async fn recieve_actionable_seek(
    seek_rx: &mut mpsc::Receiver<u64>,
    counter: Counter,
) -> Option<u64> {
    loop {
        let pos = seek_rx.recv().await?;
        if pos < counter.written() {
            return Some(pos);
        }
    }
}

enum Res3 {
    Seek(Option<u64>),
    GetRange(Result<StreamingClient, Error>),
}

enum Res4 {
    Seek(Option<u64>),
    GetClient(Result<StreamingClient, HttpError>),
}

enum StreamPartialRes {
    Canceld,
    Err(Error),
    StreamAllclient(ClientStreamingAll),
}

async fn stream_partial(
    mut client_with_stream: crate::http_client::ClientStreamingPartial,
    writer: &mut CountingWriter<impl AsyncWrite+Unpin>,
    chunk_size: u64,
    seek_rx: &mut mpsc::Receiver<u64>,
) -> StreamPartialRes {
    loop {
        let client_builder = client_with_stream.builder();
        let mut next_pos = loop {
            let stream =
                handle_partial_stream(client_with_stream, writer, chunk_size).map(Res3::GetRange);
            let get_seek = seek_rx.recv().map(Res3::Seek);
            match (stream, get_seek).race().await {
                Res3::Seek(None) => return StreamPartialRes::Canceld,
                Res3::Seek(Some(pos)) => break pos,
                Res3::GetRange(Ok(StreamingClient::Partial(client))) => client_with_stream = client,
                Res3::GetRange(Ok(StreamingClient::All(client))) => {
                    return StreamPartialRes::StreamAllclient(client)
                }
                Res3::GetRange(Err(e)) => return StreamPartialRes::Err(e),
            }
        };

        client_with_stream = loop {
            let get_seek = seek_rx.recv().map(Res4::Seek);
            let get_client_at_new_pos = client_builder
                .clone()
                .connect(next_pos, chunk_size)
                .map(Res4::GetClient);

            match (get_client_at_new_pos, get_seek).race().await {
                Res4::GetClient(Ok(StreamingClient::Partial(client))) => break client,
                Res4::GetClient(Ok(StreamingClient::All(client))) => {
                    return StreamPartialRes::StreamAllclient(client)
                }
                Res4::GetClient(Err(e)) => return StreamPartialRes::Err(Error::HttpClient(e)),
                Res4::Seek(None) => return StreamPartialRes::Canceld,
                Res4::Seek(Some(pos)) => next_pos = pos,
            }
        }
    }
}

async fn handle_partial_stream(
    client_with_stream: crate::http_client::ClientStreamingPartial,
    writer: &mut CountingWriter<impl AsyncWrite+Unpin>,
    chunk_size: u64,
) -> Result<StreamingClient, Error> {
    let mut reader = client_with_stream.into_reader();
    reader
        .read_to_writer(writer, Some(chunk_size as usize))
        .await
        .map_err(Error::HttpClient)?;
    let res = reader
        .try_into_client()
        .expect("should not read less then we requested")
        .try_get_range(writer.written() as u64, chunk_size)
        .await;
    match res {
        Ok(new_client) => Ok(new_client),
        Err(HttpError::InvalidRange) => todo!(),
        Err(e) => return Err(Error::HttpClient(e)),
    }
}
