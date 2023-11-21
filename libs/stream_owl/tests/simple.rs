use std::io::Read;
use std::sync::Arc;
use std::thread;

use futures::FutureExt;
use futures_concurrency::future::Race;

use stream_owl::testing::server;
use stream_owl::StreamBuilder;
use stream_owl::StreamCanceld;
use stream_owl::StreamError;
use stream_owl::testing::setup_tracing;
use tokio::sync::Notify;
use tokio::task::JoinError;

#[test]
fn after_seeking_forward_download_still_completes() {
    todo!()
}

#[test]
fn resume_when_server_disconnects_during_pause() {
    todo!()
}

#[test]
fn resume_when_connection_breaks_randomly() {
    todo!()
}

#[derive(Debug)]
enum Res {
    ServerCrashed(Result<Result<(), std::io::Error>, JoinError>),
    StreamCrashed(Result<StreamCanceld, StreamError>),
    TestDone,
}

async fn tomato(test_done: Arc<Notify>) -> Res {
    test_done.notified().await;
    Res::TestDone
}

#[tokio::test]
async fn seek_from_all_sides_works() {
    setup_tracing();
    let (uri, server) = server().await;

    let (mut handle, stream) = StreamBuilder::new(uri).start();
    let test_done = Arc::new(Notify::new());

    let server = server.map(Res::ServerCrashed);
    let stream = stream.map(Res::StreamCrashed);
    let done = tomato(test_done.clone());
    let crash_or_end = (server, stream, done).race();
    let crash_or_end = tokio::task::spawn(crash_or_end);

    {
        let test_done = test_done.clone();
        thread::spawn(move || {
            let mut reader = handle.try_get_reader().unwrap();
            let mut numb_buf = [0, 0, 0, 0];
            reader.read_exact(&mut numb_buf).unwrap();
            let numb = u32::from_ne_bytes(numb_buf);
            assert_eq!(numb, 0);
            test_done.notify_one();
        });
    }

    dbg!(crash_or_end.await.unwrap());
    todo!("actually turn into seek test")
}
