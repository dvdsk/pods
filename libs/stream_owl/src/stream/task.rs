use crate::http_client::Error as HttpError;
use crate::http_client::RangeRefused;
use crate::http_client::RangeSupported;
use crate::http_client::Size;
use crate::http_client::StreamingClient;
use crate::network::BandwidthLim;
use crate::network::Network;
use crate::store::StoreWriter;
use crate::target::StreamTarget;

use futures::FutureExt;
use tokio::sync::mpsc;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::Error;
use futures_concurrency::future::Race;

mod race_results;
use race_results::*;

#[derive(Debug)]
pub enum StreamDone {
    DownloadedAll,
    Canceld,
}

// the writer will limit how fast we receive data using backpressure
#[instrument(ret, skip_all)]
pub(crate) async fn new(
    url: http::Uri,
    storage: StoreWriter,
    mut seek_rx: mpsc::Receiver<u64>,
    restriction: Option<Network>,
    bandwidth_lim: BandwidthLim,
    stream_size: Size,
) -> Result<StreamDone, Error> {
    let start_pos = 0;
    let chunk_size = 1_000;
    let target = StreamTarget::new(storage, start_pos, chunk_size);

    let mut client = loop {
        let receive_seek = seek_rx.recv().map(Res1::Seek);
        let build_client = StreamingClient::new(
            url.clone(),
            restriction.clone(),
            bandwidth_lim.clone(),
            stream_size.clone(),
            &target,
        )
        .map(Res1::NewClient);

        match (build_client, receive_seek).race().await {
            Res1::NewClient(client) => break client?,
            Res1::Seek(None) => return Ok(StreamDone::Canceld),
            Res1::Seek(Some(pos)) => target.set_pos(pos),
        }
    };

    client.stream_size();

    loop {
        let client_without_range = match client {
            StreamingClient::RangesSupported(client) => {
                let res = stream_range(client, &target, &mut seek_rx).await;
                match res {
                    StreamRes::Done => return Ok(StreamDone::DownloadedAll),
                    StreamRes::Canceld => return Ok(StreamDone::Canceld),
                    StreamRes::Err(e) => return Err(e),
                    StreamRes::RefuseRange(client) => client,
                }
            }
            StreamingClient::RangesRefused(client_with_stream) => client_with_stream,
        };

        let builder = client_without_range.builder();
        let receive_seek = receive_actionable_seek(&mut seek_rx, &target).map(Res2::Seek);
        let mut reader = client_without_range.into_reader();
        let write = reader.stream_to_writer(&target, None).map(Res2::Write);

        let pos = match (write, receive_seek).race().await {
            Res2::Seek(Some(relevant_pos)) => relevant_pos,
            Res2::Seek(None) => return Ok(StreamDone::Canceld),
            Res2::Write(Err(e)) => return Err(Error::HttpClient(e)),
            Res2::Write(Ok(())) => {
                info!("At end of stream, waiting for seek");
                tracing::error!("Could be missing first part of the stream (very rare) if the server only stopped serving range requests half way through and we miss where seeking beyond the start when starting");
                stream_size.mark_stream_end(target.pos());
                match seek_rx.recv().await {
                    Some(pos) => pos,
                    None => return Ok(StreamDone::Canceld),
                }
            }
        };

        // do not concurrently wait for seeks, since we probably wont
        // get range support so any seek would translate to starting the stream
        // again. Which is wat we are doing here.
        client = builder.connect(&target).await?;
        target.set_pos(pos);
    }
}

async fn receive_actionable_seek(
    seek_rx: &mut mpsc::Receiver<u64>,
    target: &StreamTarget,
) -> Option<u64> {
    loop {
        // TODO: remove if panic does not trigger <dvdsk noreply@davidsk.dev>
        let pos = seek_rx.recv().await?;
        if pos < target.pos() {
            panic!("thought this was not needed, ooeps");
            return Some(pos);
        }
    }
}

#[derive(Debug)]
enum StreamRes {
    Done,
    Canceld,
    Err(Error),
    RefuseRange(RangeRefused),
}

#[instrument(level = "debug", skip(client, target, seek_rx), ret)]
async fn stream_range(
    mut client: RangeSupported,
    target: &StreamTarget,
    seek_rx: &mut mpsc::Receiver<u64>,
) -> StreamRes {
    loop {
        let client_builder = client.builder();

        let next_pos = loop {
            let stream = handle_partial_stream(client, target).map(Into::into);
            let get_seek = seek_rx.recv().map(Res3::Seek);
            client = match (stream, get_seek).race().await {
                Res3::Seek(None) => return StreamRes::Canceld,
                Res3::Seek(Some(pos)) => break pos,
                Res3::StreamRangesSupported(client) => client,
                Res3::StreamRangesRefused(client) => {
                    warn!("Got not seekable stream");
                    return StreamRes::RefuseRange(client);
                }
                Res3::StreamDone => return StreamRes::Done,
                Res3::StreamError(e) => return StreamRes::Err(e),
            }
        };

        target.set_pos(next_pos);
        client = loop {
            let get_seek = seek_rx.recv().map(Res4::Seek);
            let get_client_at_new_pos = client_builder.clone().connect(target).map(Into::into);

            match (get_client_at_new_pos, get_seek).race().await {
                Res4::Seek(None) => return StreamRes::Canceld,
                Res4::Seek(Some(pos)) => target.set_pos(pos),
                Res4::GetClientError(e) => return StreamRes::Err(e),
                Res4::GotRangesSupported(client) => break client,
                Res4::GotRangesRefused(client) => return StreamRes::RefuseRange(client),
            }
        }
    }
}

#[instrument(level = "debug", skip_all, ret)]
async fn handle_partial_stream(
    client: RangeSupported,
    target: &StreamTarget,
) -> Result<Option<StreamingClient>, Error> {
    let mut reader = client.into_reader();
    let max_to_stream = target.chunk_size as usize;
    reader
        .stream_to_writer(target, Some(max_to_stream))
        .await
        .map_err(Error::HttpClient)?;

    let Some(next_range) = target.next_range(&reader.stream_size()).await else {
        info!("at end of stream: returning");
        return Ok(None);
    };

    let res = reader
        .try_into_client()
        .expect("should not read less then we requested")
        .try_get_range(next_range)
        .await;

    match res {
        Ok(new_client) => Ok(Some(new_client)),
        Err(HttpError::InvalidRange) => todo!(),
        Err(e) => return Err(Error::HttpClient(e)),
    }
}
