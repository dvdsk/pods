use futures::FutureExt;
use futures::StreamExt;
use futures_concurrency::prelude::*;
use std::io::Read;
use std::pin::pin;
use stream_owl::StreamId;

use stream_owl::{ManagerError, StreamError, Manager};

use tokio::task::{self, JoinError};

const NAME1: &str = "274- The Age of the Algorithm";
const URL1: &str = "https://dts.podtrac.com/redirect.mp3/chrt.fm/track/288D49/stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/c660ce6b-ced1-459f-9535-113c670e83c9/audio/128/default.mp3?aid=rss_feed&awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&awEpisodeId=c660ce6b-ced1-459f-9535-113c670e83c9&feed=BqbsxVfO";

const NAME2: &str = "382- The ELIZA Effect";
const URL2: &str = "https://dts.podtrac.com/redirect.mp3/chrt.fm/track/288D49/stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/2099b962-5a99-4602-a67c-f99e97231227/audio/128/default.mp3?aid=rss_feed&awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&awEpisodeId=2099b962-5a99-4602-a67c-f99e97231227&feed=BqbsxVfO";

enum Res {
    ManagerFailed(ManagerError),
    StreamFailed { id: StreamId, error: StreamError },
    ReadFailed(Result<(), JoinError>),
}

impl Res {
    fn from_errors(item: Option<(StreamId, StreamError)>) -> Self {
        let (id, error) = item.unwrap();
        Self::StreamFailed { id, error }
    }
}

#[tokio::main]
async fn main() {
    let prefetch = 0;
    let mut streams = Vec::new();
    let (mut manager, manage_task, mut errors) = Manager::new(prefetch);
    streams.push(manager.add_stream_to_disk(URL1));
    streams.push(manager.add_stream_to_mem(URL2));

    streams[0].set_priority(1);
    streams[1].set_priority(0);

    let mut stream = streams.pop().unwrap();
    let do_read = task::spawn_blocking(move || {
        // because we get a reader here the corrosponding
        // handle gets the highest priority now ignoring
        // the priority set above
        let mut reader = stream.try_get_reader().expect("reader already taken");

        let mut buf = vec![0u8; 1024];
        reader.read(&mut buf).unwrap();
        assert!(buf.iter().filter(|i| **i == 0).count() < 100);
    });

    let res = (
        manage_task.map(Res::ManagerFailed),
        errors.recv().map(Res::from_errors),
        do_read.map(Res::ReadFailed),
    )
        .race()
        .await;

    match res {
        Res::ManagerFailed(e) => panic!("stream manager failed: {e:?}"),
        Res::StreamFailed { id, error } => eprintln!("stream {id:?} failed with error: {error:?}"),
        Res::ReadFailed(e) => panic!("read failed with error: {e:?}"),
    }
}
