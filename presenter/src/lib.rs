use std::error::Error;
use std::sync::mpsc;

pub use traits::{AppUpdate, UserIntent};


pub trait Ui: Send {
    fn run(&mut self) -> Result<(), Box<dyn Error>>;
}

/// converts app updates that contain various objects (DateTime
/// floats/ints) to strings that are used by the guis
pub struct Presenter {
    update_tx: mpsc::Sender<AppUpdate>,
    intent_rx: mpsc::Receiver<UserIntent>,
}

pub type Interface = (mpsc::Sender<UserIntent>, mpsc::Receiver<AppUpdate>);

pub fn new(ui_fn: Box<dyn Fn(Interface) -> Box<dyn Ui>>) -> (Box<dyn Ui>, Presenter) {
    let (update_tx, update_rx) = mpsc::channel();
    let (intent_tx, intent_rx) = mpsc::channel();
    let ui = ui_fn((intent_tx, update_rx));

    (
        ui,
        Presenter {
            update_tx,
            intent_rx,
        },
    )
}

impl traits::ClientInterface for Presenter {
    fn update(&mut self, msg: AppUpdate) {
        self.update_tx.send(msg).unwrap();
    }

    fn next_intent(&mut self) -> UserIntent {
        self.intent_rx.recv().unwrap()
    }
}
