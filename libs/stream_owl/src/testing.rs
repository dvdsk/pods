use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::thread;

use futures::FutureExt;
use futures_concurrency::future::Race;
use tokio::runtime::Runtime;
use tokio::sync::Notify;
use tokio::task::{JoinError, JoinHandle};

use crate::{StreamBuilder, StreamCanceld, StreamError, StreamHandle};

mod pausable_server;
pub use pausable_server::{pausable_server, PauseControls};

mod static_file_server;
pub use static_file_server::static_file_server;

pub fn gen_file_path() -> PathBuf {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    let mut rng = thread_rng();
    let mut name = "stream_owl_test_download_".to_owned();
    name.extend((0..8).map(|_| rng.sample(Alphanumeric) as char));

    let mut dir = std::env::temp_dir();
    dir.push(name);
    dir
}

pub fn test_data(bytes: u32) -> Vec<u8> {
    (0..bytes)
        .into_iter()
        .step_by(4)
        .flat_map(|n| n.to_ne_bytes())
        .collect()
}

pub fn setup_reader_test(
    test_done: &Arc<Notify>,
    test_file_size: u32,
    configure: impl FnOnce(StreamBuilder<false>) -> StreamBuilder<true> + Send + 'static,
    server: impl FnOnce(u64) -> (http::Uri, JoinHandle<Result<(), std::io::Error>>) + Send + 'static,
) -> (thread::JoinHandle<TestEnded>, StreamHandle) {
    setup_tracing();
    let (runtime_thread, handle) = {
        let test_done = test_done.clone();
        let (tx, rx) = mpsc::channel();
        let runtime_thread = thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let (uri, server) = server(test_file_size as u64);

                let builder = StreamBuilder::new(uri);
                let (handle, stream) = configure(builder).start().await;
                tx.send(handle).unwrap();

                let server = server.map(TestEnded::ServerCrashed);
                let stream = stream.map(TestEnded::StreamReturned);
                let done = wait_for_test_done(test_done);
                (server, stream, done).race().await
            })
        });
        let handle = rx.recv().unwrap();
        (runtime_thread, handle)
    };
    (runtime_thread, handle)
}

#[derive(Debug)]
pub enum TestEnded {
    ServerCrashed(Result<Result<(), std::io::Error>, JoinError>),
    StreamReturned(Result<StreamCanceld, StreamError>),
    TestDone,
}

async fn wait_for_test_done(test_done: Arc<Notify>) -> TestEnded {
    test_done.notified().await;
    TestEnded::TestDone
}

pub fn setup_tracing() {
    use tracing_subscriber::filter;
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;

    let filter = filter::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        filter::EnvFilter::builder()
            .parse("stream_owl=trace,tower=debug,info")
            .unwrap()
    });

    let fmt = fmt::layer()
        .pretty()
        .with_line_number(true)
        .with_test_writer();

    let _ignore_err = tracing_subscriber::registry()
        .with(filter)
        .with(fmt)
        .try_init();
}
