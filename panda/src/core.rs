use traits::{AppUpdate, UserIntent};

use crate::Reason;

/// returns when local ui intents to switch to remote
pub(super) async fn run(interface: &mut dyn traits::RemoteUI) -> Reason {
    let (tx, rx, remote) = interface.ports();
    loop {
        // get reciever for all the clients

        // join on reciever?
        match rx.next_intent().await.unwrap() {
            UserIntent::DisconnectRemote => unreachable!(),
            UserIntent::ConnectToRemote => return Reason::ConnectChange,
            UserIntent::RefuseRemoteClients => remote.disable().await,
            UserIntent::Exit => {
                tx.update(AppUpdate::Exit).await.unwrap();
                return Reason::Exit;
            }
        }
    }
}

/// returns true when we should exit
pub(super) async fn run_remote(local: &mut dyn traits::LocalUI, server: traits::Server) -> Reason {
    let (tx, rx) = local.ports();
    match rx.next_intent().await.unwrap() {
        UserIntent::Exit => Reason::Exit,
        UserIntent::ConnectToRemote => unreachable!(),
        UserIntent::RefuseRemoteClients => unreachable!(),
        UserIntent::DisconnectRemote => Reason::ConnectChange,
    }
}
