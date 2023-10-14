use std::collections::HashMap;

use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use futures::FutureExt;
use futures_concurrency::future::Race;
use tokio::sync::mpsc;

use traits::{EpisodeId, Source};

use super::memory::ToMem;
use super::Stream;

mod status;
pub(super) mod stream;

/// start a greedy remembering stream to disk.
/// prioritizes data in front of the current position
/// will download everything eventually
pub(crate) struct ToDisk {}

pub(crate) struct DiskSource {}

impl Source for DiskSource {
    fn seek(&mut self, pos: u64) {
        todo!()
    }
}

impl ToDisk {
    pub fn as_source(&self) -> Box<dyn Source> {
        todo!()
    }

    pub(crate) fn new(tx: &mut mpsc::Sender<stream::New>, url: url::Url) -> Self {
        tx.try_send(stream::New { url }).unwrap();
        Self {}
    }

    pub(crate) fn to_mem(self) -> ToMem {
        todo!()
    }

    pub(crate) fn is_playing(&self) -> bool {
        todo!()
    }
}

pub(super) fn load_streams() -> HashMap<EpisodeId, Stream> {
    status::load().iter().map(|_| todo!()).collect()
}

#[derive(Debug)]
struct StreamErr;

pub(crate) async fn stream_manager(mut rx: mpsc::Receiver<stream::New>) -> () {
    let mut streams = FuturesUnordered::new();
    enum Res {
        NewStream(stream::New),
        StreamErr(StreamErr),
        StreamDone,
        StreamEmpty,
    }

    loop {
        let new_stream = rx
            .recv()
            .map(|res| res.expect("stream manager never closes"))
            .map(Res::NewStream);

        let res = if streams.is_empty() {
            new_stream.await
        } else {
            let stream_progress = streams.next().map(|res| match res {
                Some(Ok(())) => Res::StreamDone,
                Some(Err(e)) => Res::StreamErr(e),
                None => Res::StreamEmpty,
            });
            (new_stream, stream_progress).race().await
        };


        match res {
            Res::NewStream(new) => {
                let stream = stream::process(new);
                streams.push(stream);
            }
            Res::StreamErr(e) => todo!("handle stream error like: {e:?}"),
            Res::StreamDone => (),
            Res::StreamEmpty => {
                unreachable!("endless task in streams, should never be empty")
            }
        }
    }
}
