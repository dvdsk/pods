use std::io::Read;
use std::sync::Arc;
use std::time::{Duration, Instant};

use stream_owl::testing::TestEnded;
use stream_owl::{testing, Bandwidth, StreamBuilder};
use tokio::sync::Notify;

#[test]
fn download_not_faster_then_limit() {
    let configure = {
        move |b: StreamBuilder<false>| {
            b.with_prefetch(0)
                .to_unlimited_mem()
                .with_bandwidth_limit(Bandwidth::bytes(5_000).unwrap())
        }
    };

    let test_file_size = 10_000u32;
    let test_done = Arc::new(Notify::new());

    let start = Instant::now();
    let (runtime_thread, mut handle) = {
        testing::setup_reader_test(&test_done, test_file_size, configure, move |size| {
            testing::static_file_server(size)
        })
    };

    let mut reader = handle.try_get_reader().unwrap();
    reader.read_exact(&mut vec![0; 10_000]).unwrap();

    assert!(start.elapsed() > Duration::from_secs(2));

    test_done.notify_one();
    let test_ended = runtime_thread.join().unwrap();
    assert!(matches!(test_ended, TestEnded::TestDone));
}
