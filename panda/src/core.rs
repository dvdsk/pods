use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::instrument;
use traits::{AppUpdate, DataStore, IndexSearcher, UserIntent};

use crate::Reason;

mod task;

/// returns when local ui intents to switch to remote
#[instrument(skip_all, ret)]
pub(super) async fn run(
    interface: &mut dyn traits::RemoteUI,
    searcher: Arc<Mutex<Box<dyn IndexSearcher>>>,
    db: &mut dyn DataStore,
) -> Reason {
    let mut tasks = task::Tasks::new(searcher);
    let (_, rx, remote) = interface.ports();

    loop {
        // Note different intents can be from different users
        // if we are running as a server
        let (intent, mut tx) = match rx.next_intent().await {
            Some(val) => val,
            None => return Reason::Exit,
        };

        match intent {
            UserIntent::DisconnectRemote => unreachable!(),
            UserIntent::ConnectToRemote => return Reason::ConnectChange,
            UserIntent::RefuseRemoteClients => remote.disable().await,
            UserIntent::Exit => {
                let _ignore = tx.update(AppUpdate::Exit).await;
                return Reason::Exit;
            }
            UserIntent::FullSearch { query } => tasks.search(query, tx),
            UserIntent::AddPodcast(podcast) => tasks.add_podcast(podcast, tx),
        }
    }
}

/// returns true when we should exit
#[instrument(skip_all, ret)]
pub(super) async fn run_remote(
    local: &mut dyn traits::LocalUI,
    server: traits::Server,
    db: &mut dyn DataStore,
) -> Reason {
    let (tx, rx) = local.ports();
    loop {
        let (intent, updater) = match rx.next_intent().await {
            Some(val) => val,
            None => return Reason::Exit,
        };

        match intent {
            UserIntent::Exit => return Reason::Exit,
            UserIntent::ConnectToRemote => unreachable!(),
            UserIntent::RefuseRemoteClients => unreachable!(),
            UserIntent::DisconnectRemote => return Reason::ConnectChange,
            UserIntent::FullSearch { .. } => todo!(),
            UserIntent::AddPodcast(_) => todo!(),
        }
    }
}
