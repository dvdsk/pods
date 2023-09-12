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
    intent: IntentReciever,
}

impl Drop for Interface {
    fn drop(&mut self) {
        self.listener.abort()
    }
}

#[derive(Debug)]
struct IntentReciever {
    rx: mpsc::Receiver<UserIntent>,
    tx: broadcast::Sender<AppUpdate>,
}

#[async_trait]
impl traits::IntentReciever for IntentReciever {
    async fn next_intent(&mut self) -> Option<(traits::UserIntent, Box<dyn traits::Updater>)> {
        let intent = self.rx.recv().await?;
        let updater = Box::new(self.tx.clone());
        Some((intent, updater))
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
    async fn enable(&mut self, _config: traits::Remote) {
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
    _intent_tx: mpsc::Sender<UserIntent>,
    _update_rx: broadcast::Receiver<AppUpdate>,
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
    let (intent_tx, intent_rx) = mpsc::channel(4);
    let (config_tx, config_rx) = mpsc::channel(1);
    let intent = IntentReciever {
        rx: intent_rx,
        tx: update.clone(),
    };
    let listen = listen(config_rx, intent_tx, update_rx);
    let listener = task::spawn(listen);
    let controller = RemoteController {
        sender: config_tx,
        config: init_remote,
    };

    Interface {
        intent,
        update,
        listener,
        controller,
    }
}
