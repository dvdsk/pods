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
    state: Box<dyn DataStore>,
    mut local_ui: Option<Box<dyn traits::LocalUI>>,
    mut remote: Box<dyn traits::RemoteUI>,
    searcher: Arc<Mutex<Box<dyn traits::IndexSearcher>>>,
) {
    loop {
        let server = state.settings().server().get_value();
        match (server, local_ui.as_mut()) {
            (Some(server), Some(local_ui)) => {
                match core::run_remote(local_ui.as_mut(), server).await {
                    Reason::Exit => break,
                    Reason::ConnectChange => continue,
                }
            }
            _ => (),
        }

        match local_ui {
            None => match core::run(remote.as_mut(), searcher.clone()).await {
                Reason::Exit => break,
                Reason::ConnectChange => unreachable!(),
            },
            Some(ref mut local_ui) => {
                let mut interface = interface::Unified::new(local_ui, &mut remote);
                match core::run(&mut interface, searcher.clone()).await {
                    Reason::Exit => break,
                    Reason::ConnectChange => continue,
                }
            }
        }
    }
}
