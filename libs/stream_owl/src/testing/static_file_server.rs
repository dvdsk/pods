use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Mutex;

use axum::routing::get_service;
use axum::Router;
use http::Uri;
use tokio::task::JoinHandle;
use tower_http::services::ServeFile;
use tower_http::trace::TraceLayer;

use crate::testing::test_data;

fn gen_file_if_not_there(len: u64) -> PathBuf {
    static PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

    if let Some(ref path) = *PATH.lock().unwrap() {
        if path.metadata().unwrap().len() == len {
            return path.clone();
        }
    }

    let mut dir = std::env::temp_dir();
    dir.push("stream_owl_test_source_.data");
    let path = dir;
    *PATH.lock().unwrap() = Some(path.clone());

    if path.is_file() {
        if path.metadata().unwrap().len() == len {
            return path;
        }
    }

    std::fs::write(&path, test_data(len as u32)).unwrap();
    path
}

/// # Panics
/// Must be run within a tokio runtime, if it does not this fn will panic
pub fn static_file_server(test_file_size: u64) -> (Uri, JoinHandle<Result<(), std::io::Error>>) {
    let test_data_path = gen_file_if_not_there(test_file_size);
    let serve_file = ServeFile::new(test_data_path);
    let app = Router::new().route("/stream_test", get_service(serve_file));

    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    // fn this can not be async since then we can not pass this function
    // to setup_reader_test (impl trait forbidden in fn trait return type)
    // therefore we jump through from_std
    let listener = std::net::TcpListener::bind(addr).unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = axum::serve(listener, app.layer(TraceLayer::new_for_http()));
    let server = tokio::task::spawn(server);

    let uri: Uri = format!("http://localhost:{port}/stream_test")
        .parse()
        .unwrap();

    tracing::debug!("testserver listening on on {}", uri);
    (uri, server)
}
