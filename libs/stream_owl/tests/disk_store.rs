use std::io::Seek;
use std::sync::Arc;

use stream_owl::testing::PauseControls;
use stream_owl::{testing, StreamBuilder, StreamCanceld};
use tokio::sync::Notify;

#[test]
fn after_seeking_forward_download_still_completes() {
    let test_dl_path = stream_owl::testing::gen_file_path();
    let configure = {
        let path = test_dl_path.clone();
        move |b: StreamBuilder<false>| b.with_prefetch(0).to_disk(path)
    };

    let controls = PauseControls::new();
    let test_file_size = 10_000u32;
    let test_done = Arc::new(Notify::new());

    let (runtime_thread, mut handle) = {
        let controls = controls.clone();
        testing::setup_reader_test(&test_done, test_file_size, configure, move |size| {
            testing::pausable_server(size, controls)
        })
    };

    let mut reader = handle.try_get_reader().unwrap();
    reader.seek(std::io::SeekFrom::Start(2_000)).unwrap();
    controls.unpause();

    let test_ended = runtime_thread.join().unwrap();
    match test_ended {
        testing::TestEnded::StreamReturned(Ok(StreamCanceld)) => (),
        other => panic!("runtime should return with StreamReturned, it returned with {other:?}"),
    }

    let downloaded = std::fs::read(test_dl_path).unwrap();
    assert_eq!(downloaded, testing::test_data(test_file_size as u32));
}
