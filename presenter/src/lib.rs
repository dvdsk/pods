use async_trait::async_trait;
use color_eyre::eyre;
use tokio::sync::mpsc;

pub use traits::{AppUpdate, UserIntent};

#[async_trait]
pub trait Ui: Send {
    async fn run(&mut self) -> Result<(), eyre::Report>;
}

/// converts app updates that contain various objects (DateTime
/// floats/ints) to strings that are used by the guis
pub struct Presenter {
    update_rx: mpsc::Receiver<AppUpdate>,
}

#[derive(Debug)]
pub struct Interface {
    tx: mpsc::Sender<AppUpdate>,
    rx: traits::LocalIntentReciever,
}

impl Interface {
    fn new(tx: mpsc::Sender<AppUpdate>, rx: mpsc::Receiver<UserIntent>) -> Self {
        Interface {
            rx: traits::LocalIntentReciever::new(rx, tx.clone()),
            tx,
        }
    }
}

impl traits::LocalUI for Interface {
    fn ports(&mut self) -> (&mut dyn traits::Updater, &mut dyn traits::IntentReciever) {
        (&mut self.tx, &mut self.rx)
    }
}

pub struct InternalPorts(pub ActionDecoder, pub Presenter);

pub fn new(
    ui_fn: Box<dyn Fn(InternalPorts) -> Box<dyn Ui>>,
) -> (Box<dyn Ui>, Box<dyn traits::LocalUI>) {
    let (update_tx, update_rx) = mpsc::channel(32);
    let (intent_tx, intent_rx) = mpsc::channel(32);
    let decoder = ActionDecoder { intent_tx };
    let presenter = Presenter { update_rx };
    let ui = ui_fn(InternalPorts(decoder, presenter));
    let interface = Box::new(Interface::new(update_tx, intent_rx));

    (ui, interface)
}

#[derive(Debug)]
pub enum GuiUpdate {
    Exit,
}

impl Presenter {
    pub async fn update(&mut self) -> GuiUpdate {
        let app_update = self
            .update_rx
            .recv()
            .await
            .expect("Interface should not drop before gui closes");
        match app_update {
            AppUpdate::Exit => GuiUpdate::Exit,
        }
    }
}

#[derive(Debug)]
pub enum UserAction {
    KeyPress(char),
    WindowClosed,
    SearchEnter(String),
}

pub struct ActionDecoder {
    intent_tx: mpsc::Sender<UserIntent>,
}

impl ActionDecoder {
    pub async fn decode(&mut self, action: UserAction) {
        let intent = match action {
            UserAction::KeyPress(k) if k == 'q' => UserIntent::Exit,
            UserAction::KeyPress(k) => {
                tracing::warn!("unhandled key: {k:?}");
                return;
            }
            UserAction::WindowClosed => UserIntent::Exit,
            UserAction::SearchEnter(term) => UserIntent::FullSearch(term),
        };

        self.intent_tx.send(intent).await.unwrap();
    }
}
