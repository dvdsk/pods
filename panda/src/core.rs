use async_trait::async_trait;
use traits::{AppUpdate, UserIntent};

use crate::{InterfaceClient, Reason};

#[async_trait]
pub trait Interface: Send {
    async fn next_intent(&mut self) -> UserIntent;
    async fn update(&mut self, update: AppUpdate);
}

/// returns when local ui intents to switch to remote
pub(super) async fn run(interface: &mut dyn Interface) -> Reason {
    loop {
        // get reciever for all the clients

        // join on reciever?
        match interface.next_intent().await {
            UserIntent::DisconnectRemote => unreachable!(),
            UserIntent::ConnectToRemote => return Reason::ConnectChange,
            UserIntent::Exit => {
                interface.update(AppUpdate::Exit).await;
                return Reason::Exit;
            }
        }
    }
}

/// returns true when we should exit
pub(super) async fn run_remote(local: &mut InterfaceClient, server: traits::Server) -> Reason {
    let (rx, tx) = local;
    match rx.recv().await.unwrap() {
        UserIntent::Exit => Reason::Exit,
        UserIntent::ConnectToRemote => unreachable!(),
        UserIntent::DisconnectRemote => Reason::ConnectChange,
    }
}
