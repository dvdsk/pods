use data::Data;
use presenter::{AppUpdate, UserIntent};
use std::sync::Arc;
use tokio::sync::Mutex;
use traits::{DataStore, IndexSearcher, LocalIntentReciever, Settings};

use super::simulate_user::{Action, Condition, Steps};

pub struct SimulatedUI {
    intent_tx: mpsc::Sender<UserIntent>,
    update_rx: mpsc::Receiver<AppUpdate>,
    steps: Steps,
}

use tokio::sync::mpsc;
struct SimulatedUIPorts {
    update_tx: mpsc::Sender<AppUpdate>,
    intent_reciever: traits::LocalIntentReciever,
}

pub struct State;

impl SimulatedUI {
    fn new(steps: Steps) -> (SimulatedUI, Box<dyn traits::LocalUI>) {
        let (intent_tx, intent_rx) = mpsc::channel(100);
        let (update_tx, update_rx) = mpsc::channel(100);
        let intent_reciever = LocalIntentReciever::new(intent_rx, update_tx.clone());
        (
            SimulatedUI {
                intent_tx,
                update_rx,
                steps,
            },
            Box::new(SimulatedUIPorts {
                update_tx,
                intent_reciever,
            }),
        )
    }
    async fn run(self) -> State {
        for (condition, action) in self.steps.list {
            match condition {
                Condition::None => (),
                _ => todo!(),
            }
            match action {
                Action::Intent(intent) => self.intent_tx.try_send(intent).unwrap(),
                Action::Stop => break,
                Action::View(data) => self.
            }
        }
        State
    }
}

impl traits::LocalUI for SimulatedUIPorts {
    fn ports(&mut self) -> (&mut dyn traits::Updater, &mut dyn traits::IntentReciever) {
        (&mut self.update_tx, &mut self.intent_reciever)
    }
}

impl Steps {
    pub async fn run(self) -> State {
        let mut data = Data::new();
        let server_config = data.settings_mut().server().get_value();

        let (ui, ui_port) = SimulatedUI::new(self);

        let remote = Box::new(remote_ui::new(server_config));
        let searcher = Arc::new(Mutex::new(search::new())) as Arc<Mutex<dyn IndexSearcher>>;

        let data = Box::new(data) as Box<dyn DataStore>;
        let feed = Box::new(feed::Feed::new());
        tokio::task::spawn(panda::app(data, Some(ui_port), remote, searcher, feed));

        ui.run().await
    }
}
