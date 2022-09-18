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

pub type Interface = (ActionDecoder, Presenter);

pub fn new(
    ui_fn: Box<dyn Fn(Interface) -> Box<dyn Ui>>,
) -> (
    Box<dyn Ui>,
    mpsc::Receiver<UserIntent>,
    mpsc::Sender<AppUpdate>,
) {
    let (update_tx, update_rx) = mpsc::channel(32);
    let (intent_tx, intent_rx) = mpsc::channel(32);
    let decoder = ActionDecoder { intent_tx };
    let presenter = Presenter { update_rx };
    let ui = ui_fn((decoder, presenter));

    (ui, intent_rx, update_tx)
}

#[derive(Debug)]
pub enum GuiUpdate {
    Exit
}

impl Presenter {
    pub async fn update(&mut self) -> GuiUpdate {
        let app_update = self.update_rx.recv().await.unwrap();
        match app_update {
            AppUpdate::Exit => GuiUpdate::Exit,
        }
    }
}

#[derive(Debug)]
pub enum UserAction {
    KeyPress(char),
}

pub struct ActionDecoder {
    intent_tx: mpsc::Sender<UserIntent>,
}

impl ActionDecoder {
    pub async fn decode(&mut self, action: UserAction) {
        let intent = match action {
            UserAction::KeyPress(k) if k == 'q' => UserIntent::Exit,
            UserAction::KeyPress(k) => return,
        };

        self.intent_tx.send(intent).await.unwrap();
    }
}
