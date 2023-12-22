use std::io::{Seek, Read};
use std::sync::Arc;

use stream_owl::testing::{Action, Controls, Event, TestEnded};
use stream_owl::{testing, StreamBuilder, StreamDone, StreamError};
use tokio::sync::Notify;

#[test]
fn after_seeking_forward_download_still_completes() {
    let test_dl_path = stream_owl::testing::gen_file_path();
    let configure = {
        let path = test_dl_path.clone();
        move |b: StreamBuilder<false>| b.with_prefetch(0).to_disk(path)
    };

    let controls = Controls::new();
    controls.push(Event::Any, Action::Pause);
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
        testing::TestEnded::StreamReturned(Ok(StreamDone::DownloadedAll)) => (),
        other => panic!("runtime should return with StreamReturned, it returned with {other:?}"),
    }

    let downloaded = std::fs::read(test_dl_path).unwrap();
    assert_eq!(downloaded, testing::test_data(test_file_size as u32));
}

#[test]
fn resumes() {
    let test_dl_path = stream_owl::testing::gen_file_path();
    let configure = {
        let path = test_dl_path.clone();
        move |b: StreamBuilder<false>| b.with_prefetch(0).to_disk(path)
    };

    let controls = Controls::new();
    controls.push(Event::Any, Action::Pause);
    controls.push(Event::ByteRequested(5000), Action::Cut { at: 5000 });
    controls.push(Event::Any, Action::Crash);

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
        testing::TestEnded::StreamReturned(Err(StreamError::HttpClient(
            stream_owl::http_client::Error::SendingRequest(_),
        ))) => (),
        other => panic!("runtime should return with error as stream was crashed"),
    }

    let controls = Controls::new();
    controls.push(Event::Any, Action::Pause);

    let test_file_size = 10_000u32;
    let test_done = Arc::new(Notify::new());

    let configure = {
        let path = test_dl_path.clone();
        move |b: StreamBuilder<false>| b.with_prefetch(0).to_disk(path)
    };
    let (runtime_thread, mut handle) = {
        let controls = controls.clone();
        testing::setup_reader_test(&test_done, test_file_size, configure, move |size| {
            testing::pausable_server(size, controls)
        })
    };

    let mut reader = handle.try_get_reader().unwrap();
    reader.seek(std::io::SeekFrom::Start(2_000)).unwrap();
    let mut buf = vec![0u8; 3_000];
    reader.read_exact(&mut buf).unwrap();

    assert_eq!(buf, testing::test_data_range(2_000..5_000));
    test_done.notify_one();

    let test_ended = runtime_thread.join().unwrap();
    assert!(matches!(test_ended, TestEnded::TestDone));
}
