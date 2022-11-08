use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::instrument;
use traits::{AppUpdate, IndexSearcher, SearchResult, UserIntent};

use crate::Reason;

async fn search(
    searcher: Arc<Mutex<Box<dyn IndexSearcher>>>,
    query: String,
) -> (
    Vec<SearchResult>,
    Result<(), Box<dyn std::error::Error + Send>>,
) {
    let mut searcher = searcher.lock().await;
    let val = searcher.search(&query).await;
    val
}

/// returns when local ui intents to switch to remote
#[instrument(skip_all, ret)]
pub(super) async fn run(
    interface: &mut dyn traits::RemoteUI,
    searcher: Arc<Mutex<Box<dyn IndexSearcher>>>,
) -> Reason {
    let mut tasks = JoinSet::new();
    let (tx, rx, remote) = interface.ports();

    loop {
        let (intent, updater) = match rx.next_intent().await {
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
            UserIntent::FullSearch { query, awnser } => {
                let search = search(searcher.clone(), query);
                tasks.spawn(search);
            }
        }
    }
}

/// returns true when we should exit
#[instrument(skip_all, ret)]
pub(super) async fn run_remote(local: &mut dyn traits::LocalUI, server: traits::Server) -> Reason {
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
            UserIntent::FullSearch { query, awnser } => todo!(),
        }
    }
}
