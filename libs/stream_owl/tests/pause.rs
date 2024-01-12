use std::io::Read;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use stream_owl::testing::TestEnded;
use stream_owl::{testing, StreamBuilder};
use tokio::sync::Notify;

#[test]
fn reader_only_makes_progress_after_unpause() {
    let configure =
        { move |b: StreamBuilder<false>| b.with_prefetch(0).to_unlimited_mem().start_paused(true) };

    let test_file_size = 1_000u32;
    let test_done = Arc::new(Notify::new());

    let (runtime_thread, mut handle) = {
        testing::setup_reader_test(&test_done, test_file_size, configure, move |size| {
            testing::static_file_server(size)
        })
    };

    let mut reader = handle.try_get_reader().unwrap();
    let mut buf = vec![0; 1_000];

    let reader_thread = thread::spawn(move || {
        reader.read_exact(&mut buf).unwrap();
        panic!("should never complete");
    });

    thread::sleep(Duration::from_secs(2));
    assert!(!reader_thread.is_finished());

    handle.unpause_blocking();

    thread::sleep(Duration::from_secs(2));
    assert!(reader_thread.is_finished());

    test_done.notify_one();
    let test_ended = runtime_thread.join().unwrap();
    assert!(
        matches!(test_ended, TestEnded::TestDone)
            || matches!(
                test_ended,
                TestEnded::StreamReturned(Ok(stream_owl::StreamDone::DownloadedAll))
            )
    );
}
