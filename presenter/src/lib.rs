use std::error::Error;
use std::sync::mpsc;

pub use traits::{AppUpdate, UserIntent};

pub trait Ui : Send {
    fn run(&mut self) -> Result<(), Box<dyn Error>>; 
}

pub struct Presenter {
    ui: Box<dyn Ui>,
    update_tx: mpsc::Sender<AppUpdate>,
    intent_rx: mpsc::Receiver<UserIntent>,
}

pub fn new(ui: Box<dyn Ui>) -> Presenter {
    let (update_tx, update_rx) = mpsc::channel();
    let (intent_tx, intent_rx) = mpsc::channel();

    Presenter {
        ui,
        update_tx,
        intent_rx,
    }
}

impl Presenter {
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.ui.run()
    }
}

impl traits::ClientInterface for Presenter {
    fn update(&mut self, msg: AppUpdate) {
        self.update_tx.send(msg).unwrap();
    }

    fn next_intent(&mut self) -> UserIntent {
        self.intent_rx.recv().unwrap()
    }
}
