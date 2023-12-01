use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Mutex;

use axum::{routing::get_service, Router};
use http::Uri;
use tokio::task::JoinHandle;
use tower_http::{services::ServeFile, trace::TraceLayer};

fn gen_file_if_not_there(len: u64) -> PathBuf {
    static PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

    if let Some(ref path) = *PATH.lock().unwrap() {
        if path.metadata().unwrap().len() == len {
            return path.clone();
        }
    }

    let mut dir = std::env::temp_dir();
    dir.push("stream_owl_test_file.data");
    let path = dir;
    *PATH.lock().unwrap() = Some(path.clone());

    if path.is_file() {
        if path.metadata().unwrap().len() == len {
            return path;
        }
    }

    std::fs::write(&path, test_data(1_000)).unwrap();
    path
}

fn test_data(bytes: u32) -> Vec<u8> {
    (0..bytes)
        .into_iter()
        .step_by(4)
        .map(|i| i * 4)
        .flat_map(|n| n.to_ne_bytes())
        .collect()
}

pub async fn server(test_file_size: u64) -> (Uri, JoinHandle<Result<(), std::io::Error>>) {
    let test_data_path = gen_file_if_not_there(test_file_size);
    let serve_file = ServeFile::new(test_data_path);
    let app = Router::new().route("/stream_test", get_service(serve_file));

    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tracing::debug!(
        "testserver listening on on {}",
        listener.local_addr().unwrap()
    );
    let server = axum::serve(listener, app.layer(TraceLayer::new_for_http()));
    let server = tokio::task::spawn(server);
        // Builder::new()
        // .name("test server")
        // .spawn(server)
        // .unwrap();

    let uri: Uri = format!("http://localhost:{port}/stream_test")
        .parse()
        .unwrap();
    (uri, server)
}

pub fn setup_tracing() {
    use tracing_subscriber::filter;
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;

    // console_subscriber::init();

    let filter = filter::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        filter::EnvFilter::builder()
            .parse("stream_owl=trace,info")
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
