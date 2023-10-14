use futures::FutureExt;
use futures_concurrency::prelude::*;
use std::io::Read;

use stream_owl::{ManageError, Manager, StreamError};

use tokio::task::{self, JoinError};

const URL1: &str = "https://dts.podtrac.com/redirect.mp3/chrt.fm/track/288D49/stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/c660ce6b-ced1-459f-9535-113c670e83c9/audio/128/default.mp3?aid=rss_feed&awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&awEpisodeId=c660ce6b-ced1-459f-9535-113c670e83c9&feed=BqbsxVfO";

const URL2: &str = "https://dts.podtrac.com/redirect.mp3/chrt.fm/track/288D49/stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/2099b962-5a99-4602-a67c-f99e97231227/audio/128/default.mp3?aid=rss_feed&awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&awEpisodeId=2099b962-5a99-4602-a67c-f99e97231227&feed=BqbsxVfO";

enum Res {
    ManagerFailed(ManageError),
    StreamFailed { id: usize, error: StreamError },
    ReadFailed(Result<(), JoinError>),
}

#[tokio::main]
async fn main() {
    let (mut streams, process_streams) = Manager::new();
    let (handle1, error1) = streams.add_disk(URL1);
    let (handle2, error2) = streams.add_mem(URL2);

    handle1.set_priority(1);
    handle2.set_priority(0);

    let do_read = task::spawn_blocking(move || {
        // because we get a reader here the corrosponding
        // handle gets the highest priority
        let mut reader = handle1.try_get_reader();

        let mut buf = vec![0u8; 1024];
        reader.read(&mut buf).unwrap();
        assert!(buf.iter().filter(|i| **i == 0).count() < 100);
    });

    let res = (
        process_streams.map(Res::ManagerFailed),
        error1.map(|error| Res::StreamFailed { id: 1, error }),
        error2.map(|error| Res::StreamFailed { id: 2, error }),
        do_read.map(Res::ReadFailed),
    )
        .race()
        .await;

    match res {
        Res::ManagerFailed(e) => panic!("stream manager failed: {e:?}"),
        Res::StreamFailed { id, error } => eprintln!("stream {id} failed with error: {error:?}"),
        Res::ReadFailed(e) => panic!("read failed with error: {e:?}"),
    }
}
