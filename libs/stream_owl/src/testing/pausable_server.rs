use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use http::{StatusCode, Uri};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tower_http::trace::TraceLayer;

use crate::testing::test_data;

struct PausableServer {
    test_data: Vec<u8>,
    pause_controls: PauseControls,
}

#[derive(Debug, Clone)]
pub struct PauseControls {
    paused: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl PauseControls {
    pub fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn unpause(&self) {
        self.paused.store(true, Ordering::Release);
        self.notify.notify_one();
    }
}

use axum_macros::debug_handler;
#[debug_handler]
async fn handler(
    State(state): State<Arc<PausableServer>>,
    headers: http::HeaderMap,
) -> Response<Body> {
    let range = headers.get("Range").unwrap();
    let range = range
        .to_str()
        .unwrap()
        .strip_prefix("bytes=")
        .unwrap()
        .split_once("-")
        .unwrap();
    let start = range.0.parse().unwrap();
    let stop = range.1.parse().unwrap();

    if state.pause_controls.paused.load(Ordering::Acquire) {
        state.pause_controls.notify.notified().await;
    }

    let data = state.test_data[start..stop].to_owned();
    let total = data.len();

    Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header("Content-Range", format!("bytes {start}-{stop}/{total}"))
        .header("Accept-Ranges", "bytes")
        .body(Body::from(data))
        .unwrap()
}

/// # Panics
/// Must be run within a tokio runtime, if it does not this fn will panic
pub fn pausable_server(
    test_file_size: u64,
    pause_controls: PauseControls,
) -> (Uri, JoinHandle<Result<(), std::io::Error>>) {
    let shared_state = Arc::new(PausableServer {
        test_data: test_data(test_file_size as u32),
        pause_controls,
    });

    let app = Router::new()
        .route("/stream_test", get(handler))
        .with_state(Arc::clone(&shared_state));

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
