use std::sync::Arc;

use futures::FutureExt;
use futures_concurrency::future::Race;
use tokio::sync::Mutex;
use tracing::{debug, instrument};
use traits::{AppError, AppUpdate, DataStore, Feed, IndexSearcher, Media, Player, UserIntent};

use crate::Reason;

mod task;

/// returns when local ui intents to switch to remote
#[instrument(skip_all, ret)]
pub(super) async fn run(
    interface: &mut dyn traits::RemoteUI,
    searcher: Arc<Mutex<dyn IndexSearcher>>,
    media: &mut dyn Media,
    player: &mut dyn Player,
    db: &mut dyn DataStore,
    feed: Box<dyn Feed>,
) -> Reason {
    db.set_local();
    let mut tasks = task::Tasks::new(searcher, db.writer(), db.reader());
    let db = db.writer();
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
            UserIntent::Download { episode_id } => {
                if let Some(details) = db.episode_details(episode_id) {
                    media.download(episode_id, details.url);
                } else {
                    let err = AppError::EpisodeDeleted("start download");
                    tx.update(AppUpdate::NonCriticalError(err)).await.unwrap()
                }
            }
            UserIntent::CancelDownload(episode_id) => media.cancel_download(episode_id),
            UserIntent::Play { episode_id } => {
                if let Some(details) = db.episode_details(episode_id) {
                    let source = media.get(episode_id, details.url);
                    player.play(source);
                } else {
                    let err = AppError::EpisodeDeleted("playing");
                    tx.update(AppUpdate::NonCriticalError(err)).await.unwrap()
                }
            }
        }
    }
}

/// returns true when we should exit
#[instrument(skip_all, ret)]
pub(super) async fn run_remote(
    local: &mut dyn traits::LocalUI,
    _server: traits::Server,
    db: &mut dyn DataStore,
) -> Reason {
    db.set_remote();
    let (_tx, rx) = local.ports();
    loop {
        let (intent, _updater) = match rx.next_intent().await {
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
            UserIntent::Play { .. } => todo!(),
            UserIntent::Download { .. } => todo!(),
            UserIntent::CancelDownload(_) => todo!(),
        }
    }
}
