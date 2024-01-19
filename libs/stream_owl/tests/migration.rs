use std::io::{Read, Seek};
use std::num::NonZeroUsize;
use std::sync::Arc;

use stream_owl::testing::{Action, Controls, Event, TestEnded};
use stream_owl::{testing, StreamBuilder};
use tokio::sync::Notify;

#[test]
fn migrate_to_disk() {
    let test_dl_path = stream_owl::testing::gen_file_path();
    let configure = |b: StreamBuilder<false>| {
        b.with_prefetch(0)
            .to_limited_mem(NonZeroUsize::new(2000).unwrap())
    };

    let controls = Controls::new();
    controls.push(Event::ByteRequested(3000), Action::Cut { at: 3000 });

    let test_file_size = 10_000u32;
    let test_done = Arc::new(Notify::new());

    let (runtime_thread, mut handle) = {
        let controls = controls.clone();
        testing::setup_reader_test(&test_done, test_file_size, configure, move |size| {
            testing::pausable_server(size, controls)
        })
    };

    let mut reader = handle.try_get_reader().unwrap();
    reader.seek(std::io::SeekFrom::Start(1_000)).unwrap();
    controls.unpause_all();
    reader.read_exact(&mut vec![0; 1_000]).unwrap();

    let migration = handle.use_disk_backend_blocking(test_dl_path.clone()).unwrap();
    reader.read_exact(&mut vec![0; 1_000]).unwrap();
    migration.block_till_done().unwrap();

    reader.seek(std::io::SeekFrom::Start(1_000)).unwrap();
    reader.read_exact(&mut vec![0; 2_000]).unwrap();

    test_done.notify_one();
    let test_ended = runtime_thread.join().unwrap();
    assert!(matches!(test_ended, TestEnded::TestDone));

    let downloaded = std::fs::read(test_dl_path).unwrap();
    assert_eq!(downloaded, testing::test_data(downloaded.len() as u32));
}
