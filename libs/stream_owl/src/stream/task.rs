use crate::http_client::Error as HttpError;
use crate::http_client::RangeRefused;
use crate::http_client::RangeSupported;
use crate::http_client::Size;
use crate::http_client::StreamingClient;
use crate::network::Network;
use crate::store::SwitchableStore;
use crate::target::StreamTarget;

use futures::FutureExt;
use tokio::sync::mpsc;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::Error;
use futures_concurrency::future::Race;

#[derive(Debug)]
pub struct Canceld;

enum Res1 {
    Seek(Option<u64>),
    NewClient(Result<StreamingClient, HttpError>),
}

enum Res2 {
    Seek(Option<u64>),
    Write(Result<(), HttpError>),
}

// the writer will limit how fast we receive data using backpressure
#[instrument(ret, skip_all)]
pub(crate) async fn new(
    url: http::Uri,
    storage: SwitchableStore,
    mut seek_rx: mpsc::Receiver<u64>,
    restriction: Option<Network>,
    stream_size: Size,
) -> Result<Canceld, Error> {
    let start_pos = 0;
    let chunk_size = 1_000;
    let mut target = StreamTarget::new(storage, start_pos, chunk_size);

    let mut client = loop {
        let receive_seek = seek_rx.recv().map(Res1::Seek);
        let build_client = StreamingClient::new(
            url.clone(),
            restriction.clone(),
            stream_size.clone(),
            target.clone(),
        )
        .map(Res1::NewClient);

        match (build_client, receive_seek).race().await {
            Res1::NewClient(client) => break client?,
            Res1::Seek(None) => return Ok(Canceld),
            Res1::Seek(Some(pos)) => target.set_pos(pos),
        }
    };

    client.stream_size();

    loop {
        let client_without_range = match client {
            StreamingClient::RangeSupported(client) => {
                let res = stream_range(client, target.clone(), &mut seek_rx).await;
                match res {
                    StreamRes::Canceld => return Ok(Canceld),
                    StreamRes::Err(e) => return Err(e),
                    StreamRes::RefuseRange(client) => client,
                }
            }
            StreamingClient::RangeRefused(client_with_stream) => client_with_stream,
        };

        let builder = client_without_range.builder();
        let receive_seek = receive_actionable_seek(&mut seek_rx, target.clone()).map(Res2::Seek);
        let mut reader = client_without_range.into_reader();
        let write = reader.stream_to_writer(&mut target, None).map(Res2::Write);

        let pos = match (write, receive_seek).race().await {
            Res2::Seek(Some(relevant_pos)) => relevant_pos,
            Res2::Seek(None) => return Ok(Canceld),
            Res2::Write(Err(e)) => return Err(Error::HttpClient(e)),
            Res2::Write(Ok(())) => {
                info!("At end of stream, waiting for seek");
                todo!("Could be missing first part of the stream (very rare) if the server only stopped serving range requests half way through and we miss where seeking beyond the start when starting");
                stream_size.mark_stream_end(target.pos());
                match seek_rx.recv().await {
                    Some(pos) => pos,
                    None => return Ok(Canceld),
                }
            }
        };

        // do not concurrently wait for seeks, since we probably wont
        // get range support so any seek would translate to starting the stream
        // again. Which is wat we are doing here.
        client = builder.connect(target.clone()).await?;
        target.set_pos(pos);
    }
}

async fn receive_actionable_seek(
    seek_rx: &mut mpsc::Receiver<u64>,
    target: StreamTarget,
) -> Option<u64> {
    loop {
        let pos = seek_rx.recv().await?;
        if pos < target.pos() {
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

#[derive(Debug)]
enum StreamRes {
    Canceld,
    Err(Error),
    RefuseRange(RangeRefused),
}

#[instrument(level = "debug", skip(client_with_stream, target, seek_rx), ret)]
async fn stream_range(
    mut client_with_stream: RangeSupported,
    target: StreamTarget,
    seek_rx: &mut mpsc::Receiver<u64>,
) -> StreamRes {
    loop {
        let client_builder = client_with_stream.builder();

        let next_pos = loop {
            let stream =
                handle_partial_stream(client_with_stream, target.clone()).map(Res3::GetRange);
            let get_seek = seek_rx.recv().map(Res3::Seek);
            client_with_stream = match (stream, get_seek).race().await {
                Res3::Seek(None) => return StreamRes::Canceld,
                Res3::Seek(Some(pos)) => break pos,
                Res3::GetRange(Ok(StreamingClient::RangeSupported(client))) => client,
                Res3::GetRange(Ok(StreamingClient::RangeRefused(client))) => {
                    warn!("Got not seekable stream");
                    return StreamRes::RefuseRange(client);
                }
                Res3::GetRange(Err(e)) => return StreamRes::Err(e),
            }
        };

        target.set_pos(next_pos);
        client_with_stream = loop {
            let get_seek = seek_rx.recv().map(Res4::Seek);
            let get_client_at_new_pos = client_builder
                .clone()
                .connect(target.clone())
                .map(Res4::GetClient);

            match (get_client_at_new_pos, get_seek).race().await {
                Res4::GetClient(Ok(StreamingClient::RangeSupported(client))) => break client,
                Res4::GetClient(Ok(StreamingClient::RangeRefused(client))) => {
                    return StreamRes::RefuseRange(client)
                }
                Res4::GetClient(Err(e)) => return StreamRes::Err(Error::HttpClient(e)),
                Res4::Seek(None) => return StreamRes::Canceld,
                Res4::Seek(Some(pos)) => target.set_pos(pos),
            }
        }
    }
}

#[instrument(level = "debug", skip_all, ret)]
async fn handle_partial_stream(
    client_with_stream: RangeSupported,
    mut target: StreamTarget,
) -> Result<StreamingClient, Error> {
    let mut reader = client_with_stream.into_reader();
    let max_to_stream = target.chunk_size as usize;
    reader
        .stream_to_writer(&mut target, Some(max_to_stream))
        .await
        .map_err(Error::HttpClient)?;

    let size = reader.stream_size().known();
    debug_assert!(
        size.is_some(),
        "A partial stream must have valid content-range header"
    );

    let next_range = target.next_range(size);
    if next_range.is_empty() {
        info!("at end of stream: waiting for next seek");
        let () = std::future::pending().await;
        unreachable!()
    }

    let res = reader
        .try_into_client()
        .expect("should not read less then we requested")
        .try_get_range(next_range)
        .await;

    match res {
        Ok(new_client) => Ok(new_client),
        Err(HttpError::InvalidRange) => todo!(),
        Err(e) => return Err(Error::HttpClient(e)),
    }
}
