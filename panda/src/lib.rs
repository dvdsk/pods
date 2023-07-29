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
    searcher: Arc<Mutex<Box<dyn traits::IndexSearcher>>>,
) {
    loop {
        let server = state.settings().server().get_value();
        if let Some((server, local_ui)) = server.zip(local_ui.as_mut()) {
            match core::run_remote(local_ui.as_mut(), server, state.as_mut()).await {
                Reason::Exit => break,
                Reason::ConnectChange => continue,
            }
        }

        match local_ui {
            None => match core::run(remote.as_mut(), searcher.clone(), state.as_mut()).await {
                Reason::Exit => break,
                Reason::ConnectChange => unreachable!(),
            },
            Some(ref mut local_ui) => {
                let mut interface = interface::Unified::new(local_ui, &mut remote);
                match core::run(&mut interface, searcher.clone(), state.as_mut()).await {
                    Reason::Exit => break,
                    Reason::ConnectChange => continue,
                }
            }
        }
    }
}
