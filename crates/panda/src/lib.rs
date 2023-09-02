use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::instrument;
use traits::DataStore;

mod core;
mod interface;

#[derive(Debug)]
enum Reason {
    Exit,
    ConnectChange,
}

#[instrument(skip_all)]
pub async fn app(
    mut state: Box<dyn DataStore>,
    mut local_ui: Option<Box<dyn traits::LocalUI>>,
    mut remote: Box<dyn traits::RemoteUI>,
    searcher: Arc<Mutex<dyn traits::IndexSearcher>>,
    mut media: Box<dyn traits::Media>,
    mut player: Box<dyn traits::Player>,
    feed: Box<dyn traits::Feed>,
) {
    loop {
        let server = state.reader().settings().server().get_value();
        if let Some((server, local_ui)) = server.zip(local_ui.as_mut()) {
            match core::run_remote(local_ui.as_mut(), server, state.as_mut()).await {
                Reason::Exit => break,
                Reason::ConnectChange => continue,
            }
        }

        match local_ui {
            None => {
                match core::run(
                    remote.as_mut(),
                    searcher.clone(),
                    media.as_mut(),
                    player.as_mut(),
                    state.as_mut(),
                    feed.box_clone(),
                )
                .await
                {
                    Reason::Exit => break,
                    Reason::ConnectChange => unreachable!(),
                }
            }
            Some(ref mut local_ui) => {
                let mut interface = interface::Unified::new(local_ui, &mut remote);
                match core::run(
                    &mut interface,
                    searcher.clone(),
                    media.as_mut(),
                    player.as_mut(),
                    state.as_mut(),
                    feed.box_clone(),
                )
                .await
                {
                    Reason::Exit => break,
                    Reason::ConnectChange => continue,
                }
            }
        }
    }
}
