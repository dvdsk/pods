use tokio::sync::{broadcast, mpsc};
use tokio::task;
use traits::{async_trait, AppUpdate, UserIntent};

#[derive(Debug)]
struct RemoteController {
    config: Option<traits::Server>,
    sender: mpsc::Sender<Option<traits::Server>>,
}

#[derive(Debug)]
pub struct Interface {
    listener: task::JoinHandle<()>,
    controller: RemoteController,
    update: broadcast::Sender<AppUpdate>,
    intent: mpsc::Receiver<UserIntent>,
}

impl Drop for Interface {
    fn drop(&mut self) {
        self.listener.abort()
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
        (&mut self.update, &mut self.intent, &mut self.controller)
    }
    fn controller(&mut self) -> &mut dyn traits::RemoteController {
        &mut self.controller
    }
}

#[async_trait]
impl traits::RemoteController for RemoteController {
    async fn disable(&mut self) {
        self.config = None;
        self.sender.send(self.config.clone()).await.unwrap();
    }
    async fn enable(&mut self, config: traits::Remote) {
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
    mut config_rx: mpsc::Receiver<Option<traits::Server>>,
    intent_tx: mpsc::Sender<UserIntent>,
    update_rx: broadcast::Receiver<AppUpdate>,
) {
    while let Some(change) = config_rx.recv().await {
        match change {
            Some(config) => todo!("do something with new config: {config:?}"),
            None => todo!("can not disable listen as thats not yet implemented"),
        }
    }
}

pub fn new(init_remote: Option<traits::Server>) -> Interface {
    let (update, update_rx) = broadcast::channel(4);
    let (intent_tx, intent) = mpsc::channel(4);
    let (config_tx, config_rx) = mpsc::channel(1);
    let listen = listen(config_rx, intent_tx, update_rx);
    let listener = task::spawn(listen);
    let controller = RemoteController { sender: config_tx, config: init_remote};

    Interface {
        intent,
        update,
        listener,
        controller,
    }
}
