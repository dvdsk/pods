use std::io::Read;
use std::io::Seek;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use futures::FutureExt;
use futures_concurrency::future::Race;

use stream_owl::testing::server;
use stream_owl::testing::setup_tracing;
use stream_owl::Reader;
use stream_owl::StreamBuilder;
use stream_owl::StreamCanceld;
use stream_owl::StreamError;
use stream_owl::StreamHandle;
use tokio::runtime::Runtime;
use tokio::sync::Notify;
use tokio::task::JoinError;
use tracing::info;
use tracing::instrument;

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

async fn wait_for_test_done(test_done: Arc<Notify>) -> Res {
    test_done.notified().await;
    Res::TestDone
}

#[test]
fn seek_from_all_sides_works() {
    let test_file_size = 10_000u32;
    let memory_buffer_size = 1000;
    let prefetch = 0;
    let test_done = Arc::new(Notify::new());

    let (runtime_thread, mut handle) = setup_reader_test(&test_done, test_file_size);

    let mut reader = handle.try_get_reader().unwrap();
    assert_pos(&mut reader, 0);
    reader.seek(std::io::SeekFrom::Start(40)).unwrap();
    assert_pos(&mut reader, 40);
    reader.seek(std::io::SeekFrom::Current(40)).unwrap();
    assert_pos(&mut reader, 80);
    reader.seek(std::io::SeekFrom::End(40)).unwrap();
    assert_pos(&mut reader, test_file_size - 40);
    test_done.notify_one();
    runtime_thread.join().unwrap();
}

fn setup_reader_test(
    test_done: &Arc<Notify>,
    test_file_size: u32,
) -> (thread::JoinHandle<()>, StreamHandle) {
    let (runtime_thread, handle) = {
        let test_done = test_done.clone();
        let (tx, rx) = mpsc::channel();
        let runtime_thread = thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                setup_tracing();
                let (uri, server) = server(test_file_size as u64).await;

                let (handle, stream) = StreamBuilder::new(uri).start();
                tx.send(handle).unwrap();

                let server = server.map(Res::ServerCrashed);
                let stream = stream.map(Res::StreamCrashed);
                let done = wait_for_test_done(test_done);
                let crash_or_end = (server, stream, done).race();
                let crash_or_end = tokio::task::spawn(crash_or_end);

                crash_or_end.await.unwrap();
            });
        });
        let handle = rx.recv().unwrap();
        (runtime_thread, handle)
    };
    (runtime_thread, handle)
}

#[instrument(skip(reader))]
fn assert_pos(reader: &mut Reader, bytes_from_start: u32) {
    let mut numb_buf = [0, 0, 0, 0];
    info!("hi");
    reader.read_exact(&mut numb_buf).unwrap();
    info!("hi");
    let numb = u32::from_ne_bytes(numb_buf);
    let correct = bytes_from_start / std::mem::size_of::<u32>() as u32;
    assert_eq!(
        numb, correct,
        "expected: {correct} got {numb} at {bytes_from_start} bytes from start"
    );
}
