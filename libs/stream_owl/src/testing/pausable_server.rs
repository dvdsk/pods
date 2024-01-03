use std::net::SocketAddr;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use http::{StatusCode, Uri};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tower_http::trace::TraceLayer;
use tracing::instrument;

use crate::testing::test_data;

struct ControllableServer {
    test_data: Vec<u8>,
    pause_controls: Controls,
}

#[derive(Debug, Clone)]
pub enum Event {
    Any,
    ByteRequested(u64),
}

impl Event {
    fn active(&self, range: &Range<u64>) -> bool {
        match self {
            Event::ByteRequested(n) => range.contains(n),
            Event::Any => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Cut { at: u64 },
    Crash,
    Pause,
}

impl Action {
    async fn perform(&self, state: &ControllableServer, range: &mut Range<u64>) {
        match self {
            Action::Crash => {
                panic!("Crash requested from test server")
            }
            Action::Cut { at } => range.end = *at,
            Action::Pause => {
                tracing::warn!("Test server waiting to be unpaused");
                state.pause_controls.notify.notified().await;
            }
        }
    }
}

#[derive(Debug)]
struct InnerControls {
    on_event: Vec<(Event, Action)>,
}

#[derive(Debug, Clone)]
pub struct Controls {
    inner: Arc<Mutex<InnerControls>>,
    notify: Arc<Notify>,
}

impl Controls {
    pub fn new() -> Self {
        let inner = InnerControls {
            on_event: Vec::new(),
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn push(&self, event: Event, action: Action) {
        self.inner.lock().unwrap().on_event.push((event, action))
    }

    pub fn push_front(&self, event: Event, action: Action) {
        self.inner
            .lock()
            .unwrap()
            .on_event
            .insert(0, (event, action))
    }

    /// unpauses and remove any future plain pauses
    pub fn unpause(&self) {
        tracing::warn!("Unpausing debug server");
        let mut inner = self.inner.lock().unwrap();
        let to_remove: Vec<_> = inner
            .on_event
            .iter()
            .enumerate()
            .filter(|(_, (_, action))| *action == Action::Pause)
            .map(|(idx, _)| idx)
            .collect();
        for to_remove in to_remove.into_iter().rev() {
            inner.on_event.remove(to_remove);
        }

        self.notify.notify_one();
    }
}

use axum_macros::debug_handler;
#[debug_handler]
#[instrument(level = "debug", skip(state))]
async fn handler(
    State(state): State<Arc<ControllableServer>>,
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

    let action = {
        let range = start..stop;
        let inner = state.pause_controls.inner.lock().unwrap();
        if let Some((idx, (_, _))) = inner
            .on_event
            .iter()
            .enumerate()
            .find(|(_, (event, _))| event.active(&range))
        {
            Some(inner.on_event.get(idx).unwrap().1.clone())
        } else {
            None
        }
    };

    let mut range = start..stop;
    if let Some(action) = action {
        action.perform(&state, &mut range).await;
    }

    let data = state.test_data[range.start as usize..range.end as usize].to_owned();
    let total = state.test_data.len();

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
    pause_controls: Controls,
) -> (Uri, JoinHandle<Result<(), std::io::Error>>) {
    let shared_state = Arc::new(ControllableServer {
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
