use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::http_client::ClientStreamingAll;
use crate::http_client::ClientStreamingPartial;
use crate::http_client::Error as HttpError;
use crate::http_client::Size;
use crate::http_client::StreamingClient;
use crate::network::Network;
use crate::store;
use crate::store::SwitchableStore;
use crate::Appender;

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

#[derive(Debug, Clone)]
struct StoreAppender {
    /// on seek this pos is updated, in between seeks
    /// it increments with the number of bytes written
    pos: Arc<AtomicU64>,
    store: SwitchableStore,
}

impl StoreAppender {
    fn pos(&self) -> u64 {
        self.pos.load(Ordering::Acquire)
    }
}

#[async_trait::async_trait]
impl Appender for StoreAppender {
    #[instrument(level = "trace", skip(self, buf), fields(buf_len = buf.len()))]
    async fn append(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        // only this function modifies pos,
        // only need to read threads own writes => relaxed ordering
        let written = self
            .store
            .write_at(buf, self.pos.load(Ordering::Relaxed))
            .await;

        let bytes = match written {
            Ok(bytes) => bytes.get(),
            Err(store::Error::SeekInProgress) => {
                //
                futures::pending!();
                unreachable!()
            }
            Err(other) => {
                todo!("handle other error: {other:?}")
            }
        };

        // new data needs to be requested after current pos, it uses acquire Ordering
        self.pos.fetch_add(bytes as u64, Ordering::Release);
        Ok(bytes)
    }
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
    let mut start_pos = 0;
    let chunk_size = 1_000;

    let mut client = loop {
        let receive_seek = seek_rx.recv().map(Res1::Seek);
        let build_client = StreamingClient::new(
            url.clone(),
            restriction.clone(),
            start_pos,
            chunk_size,
            stream_size.clone(),
        )
        .map(Res1::NewClient);

        match (build_client, receive_seek).race().await {
            Res1::NewClient(client) => break client?,
            Res1::Seek(None) => return Ok(Canceld),
            Res1::Seek(Some(pos)) => start_pos = pos,
        }
    };

    client.stream_size();
    let appender = StoreAppender {
        store: storage,
        pos: Arc::new(AtomicU64::new(start_pos)),
    };

    loop {
        let stream_all_client = match client {
            StreamingClient::Partial(client_with_stream) => {
                let res = stream_partial(
                    client_with_stream,
                    appender.clone(),
                    chunk_size,
                    &mut seek_rx,
                )
                .await;
                match res {
                    StreamRes::Canceld => return Ok(Canceld),
                    StreamRes::Err(e) => return Err(e),
                    StreamRes::StreamAllclient(client) => client,
                }
            }
            StreamingClient::All(client_with_stream) => client_with_stream,
        };

        let builder = stream_all_client.builder();
        let receive_seek = receive_actionable_seek(&mut seek_rx, appender.clone()).map(Res2::Seek);
        let mut reader = stream_all_client.into_reader();
        let write = reader
            .read_to_writer(appender.clone(), None)
            .map(Res2::Write);

        let pos = match (write, receive_seek).race().await {
            Res2::Seek(Some(relevant_pos)) => relevant_pos,
            Res2::Seek(None) => return Ok(Canceld),
            Res2::Write(Err(e)) => return Err(Error::HttpClient(e)),
            Res2::Write(Ok(())) => {
                info!("At end of stream, waiting for seek");
                todo!("Could be missing first part of the stream (very rare) if the server only stopped serving range requests half way through and we miss where seeking beyond the start when starting");
                stream_size.mark_stream_end(appender.pos());
                match seek_rx.recv().await {
                    Some(pos) => pos,
                    None => return Ok(Canceld),
                }
            }
        };

        // do not concurrently wait for seeks, since we probably wont
        // get range support so any seek would translate to starting the stream
        // again. Which is wat we are doing here.
        client = builder.connect(pos, chunk_size).await?;
        appender.pos.store(pos, Ordering::Relaxed);
    }
}

async fn receive_actionable_seek(
    seek_rx: &mut mpsc::Receiver<u64>,
    appender: StoreAppender,
) -> Option<u64> {
    loop {
        let pos = seek_rx.recv().await?;
        if pos < appender.pos.load(Ordering::Relaxed) {
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
    StreamAllclient(ClientStreamingAll),
}

#[instrument(level = "debug", skip(client_with_stream, appender, seek_rx), ret)]
async fn stream_partial(
    mut client_with_stream: ClientStreamingPartial,
    appender: StoreAppender,
    chunk_size: u64,
    seek_rx: &mut mpsc::Receiver<u64>,
) -> StreamRes {
    loop {
        let client_builder = client_with_stream.builder();

        let mut next_pos = loop {
            let stream = handle_partial_stream(client_with_stream, appender.clone(), chunk_size)
                .map(Res3::GetRange);
            let get_seek = seek_rx.recv().map(Res3::Seek);
            client_with_stream = match (stream, get_seek).race().await {
                Res3::Seek(None) => return StreamRes::Canceld,
                Res3::Seek(Some(pos)) => break pos,
                Res3::GetRange(Ok(StreamingClient::Partial(client))) => client,
                Res3::GetRange(Ok(StreamingClient::All(client))) => {
                    warn!("Got not seekable stream");
                    return StreamRes::StreamAllclient(client);
                }
                Res3::GetRange(Err(e)) => return StreamRes::Err(e),
            }
        };

        appender.pos.store(next_pos, Ordering::Relaxed);
        client_with_stream = loop {
            let get_seek = seek_rx.recv().map(Res4::Seek);
            let get_client_at_new_pos = client_builder
                .clone()
                .connect(next_pos, chunk_size)
                .map(Res4::GetClient);

            match (get_client_at_new_pos, get_seek).race().await {
                Res4::GetClient(Ok(StreamingClient::Partial(client))) => break client,
                Res4::GetClient(Ok(StreamingClient::All(client))) => {
                    return StreamRes::StreamAllclient(client)
                }
                Res4::GetClient(Err(e)) => return StreamRes::Err(Error::HttpClient(e)),
                Res4::Seek(None) => return StreamRes::Canceld,
                Res4::Seek(Some(pos)) => next_pos = pos,
            }
        }
    }
}

#[instrument(level = "debug", skip_all, ret)]
async fn handle_partial_stream(
    client_with_stream: ClientStreamingPartial,
    appender: StoreAppender,
    chunk_size: u64,
) -> Result<StreamingClient, Error> {
    let mut reader = client_with_stream.into_reader();
    reader
        .read_to_writer(appender.clone(), Some(chunk_size as usize))
        .await
        .map_err(Error::HttpClient)?;

    let size = reader
        .stream_size()
        .known()
        .expect("A partial range must have a valid content-range header");
    if appender.pos() >= size {
        info!("at end of stream: waiting for next seek");
        let () = std::future::pending().await;
        unreachable!()
    }

    let chunk_size = chunk_size.min(size - appender.pos());
    let res = reader
        .try_into_client()
        .expect("should not read less then we requested")
        .try_get_range(appender.pos(), chunk_size)
        .await;

    match res {
        Ok(new_client) => Ok(new_client),
        Err(HttpError::InvalidRange) => todo!(),
        Err(e) => return Err(Error::HttpClient(e)),
    }
}
