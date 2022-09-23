use tokio::sync::{broadcast, mpsc};
use tokio::task;
use traits::{async_trait, AppUpdate, UserIntent};

pub struct Listener(pub Option<task::JoinHandle<()>>);

pub struct Interface {
    listener: Listener,
    update: broadcast::Sender<AppUpdate>,
    intent: mpsc::Receiver<UserIntent>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        if let Some(ref mut task) = self.0 {
            task.abort()
        }
    }
}

#[async_trait]
impl traits::RemoteUI for Interface {
    fn ports(
        &mut self,
    ) -> (
        &mut dyn traits::Updater,
        &mut dyn traits::IntentReciever,
        &mut dyn traits::RemoteController,
    ) {
        (&mut self.update, &mut self.intent, &mut self.listener)
    }
    fn controller(&mut self) -> &mut dyn traits::RemoteController {
        &mut self.listener
    }
}

#[async_trait]
impl traits::RemoteController for Listener {
    fn disable(&mut self) {
        if let Some(task) = self.0.take() {
            task.abort()
        }
    }
    fn enable(&mut self, config: traits::Remote) {
        todo!()
    }
}

#[async_trait]
impl traits::LocalUI for Interface {
    fn ports(&mut self) -> (&mut dyn traits::Updater, &mut dyn traits::IntentReciever) {
        (&mut self.update, &mut self.intent)
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
    let listener = Listener(init_remote.map(|config| {
        let listen = listen(config, intent_tx, update_rx);
        task::spawn(listen)
    }));

    Interface {
        intent,
        update,
        listener,
    }
}
