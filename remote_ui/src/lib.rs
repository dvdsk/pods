use core::fmt;

use tokio::sync::{broadcast, mpsc};
use tokio::task;
use traits::{async_trait, AppUpdate, UserIntent};

pub struct Interface {
    listener: Option<task::JoinHandle<()>>,
    update: broadcast::Sender<AppUpdate>,
    intent: mpsc::Receiver<UserIntent>,
}

pub struct Updater {
    broadcast: broadcast::Sender<AppUpdate>,
}

#[async_trait]
impl traits::Updater for Updater {
    async fn update(&mut self, msg: AppUpdate) -> Result<(), Box<dyn fmt::Display>> {
        self.broadcast.send(msg).unwrap();
        Ok(())
    }
}

impl Drop for Interface {
    fn drop(&mut self) {
        if let Some(ref mut task) = self.listener {
            task.abort()
        }
    }
}

#[async_trait]
impl traits::RemoteUI for Interface {
    fn disable(&mut self) {
        if let Some(task) = self.listener.take() {
            task.abort()
        }
    }
    fn enable(&mut self, config: traits::Remote) {
        todo!()
    }
    fn updater(&self) -> Box<dyn traits::Updater> {
        Box::new(Updater {
            broadcast: self.update.clone(),
        })
    }
    fn intent(&self) -> Box<dyn traits::IntentReciever> {
        todo!()
    }
}

#[async_trait]
impl traits::IntentReciever for Interface {
    async fn next_intent(&mut self) -> Result<UserIntent, Box<dyn fmt::Display>> {
        todo!()
    }
}

async fn listen(
    config: traits::Remote,
    intent_tx: mpsc::Sender<UserIntent>,
    update_rx: broadcast::Receiver<AppUpdate>,
) {
    todo!()
}

pub fn new(init_remote: Option<traits::Remote>) -> Interface {
    let (update, update_rx) = broadcast::channel(4);
    let (intent_tx, intent) = mpsc::channel(4);
    let listener = init_remote.map(|config| {
        let listen = listen(config, intent_tx, update_rx);
        task::spawn(listen)
    });

    Interface {
        intent,
        update,
        listener,
    }
}
