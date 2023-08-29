use std::sync::Arc;

use futures::FutureExt;
use futures_concurrency::future::Race;
use tokio::sync::Mutex;
use tracing::{debug, instrument};
use traits::{AppUpdate, DataStore, Feed, IndexSearcher, UserIntent};

use crate::Reason;

mod task;

/// returns when local ui intents to switch to remote
#[instrument(skip_all, ret)]
pub(super) async fn run(
    interface: &mut dyn traits::RemoteUI,
    searcher: Arc<Mutex<dyn IndexSearcher>>,
    db: &mut dyn DataStore,
    feed: Box<dyn Feed>,
) -> Reason {
    db.set_local();
    let mut tasks = task::Tasks::new(searcher, db.writer(), db.reader());
    tasks.start_maintain_feed(feed).await;
    let (_, rx, remote) = interface.ports();

    loop {
        // Note different intents can be from different users
        // if we are running as a server
        enum Res {
            Intent(Option<(UserIntent, Box<(dyn traits::Updater + 'static)>)>),
            Panic(Box<dyn std::any::Any + Send + 'static>),
        }

        let catch_panics = tasks.panicked().map(Res::Panic);
        let next_intent = rx.next_intent().map(Res::Intent);
        let res = (catch_panics, next_intent).race().await;

        let next_intent = match res {
            Res::Panic(reason) => std::panic::resume_unwind(reason),
            Res::Intent(next_intent) => next_intent,
        };

        let (intent, mut tx) = match next_intent {
            Some(val) => val,
            None => return Reason::Exit,
        };

        debug!("got intent: {intent:?}");
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
    db.set_remote();
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
