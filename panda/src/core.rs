use tracing::instrument;
use traits::{AppUpdate, UserIntent};

use crate::Reason;

/// returns when local ui intents to switch to remote
#[instrument(skip_all, ret)]
pub(super) async fn run(interface: &mut dyn traits::RemoteUI) -> Reason {
    let (tx, rx, remote) = interface.ports();
    loop {
        let intent = match rx.next_intent().await {
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
            UserIntent::FullSearch(s) => todo!(),
        }
    }
}

/// returns true when we should exit
#[instrument(skip_all, ret)]
pub(super) async fn run_remote(local: &mut dyn traits::LocalUI, server: traits::Server) -> Reason {
    let (tx, rx) = local.ports();
    match rx.next_intent().await.unwrap() {
        UserIntent::Exit => Reason::Exit,
        UserIntent::ConnectToRemote => unreachable!(),
        UserIntent::RefuseRemoteClients => unreachable!(),
        UserIntent::DisconnectRemote => Reason::ConnectChange,
        UserIntent::FullSearch(s) => todo!(),
    }
}
