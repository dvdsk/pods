use std::pin::Pin;

use async_trait::async_trait;
use color_eyre::eyre;
use futures::StreamExt;
use futures_core::stream::Stream;
use tokio::sync::{mpsc, oneshot};

use tracing::instrument;
use traits::DataRStore;
use traits::DataUpdate;
use traits::ReqUpdate;
use traits::SearchResult;
pub use traits::{AppUpdate, UserIntent};

mod tasks;

#[async_trait]
pub trait Ui: Send {
    async fn run(&mut self) -> Result<(), eyre::Report>;
}

/// converts app updates that contain various objects (DateTime
/// floats/ints) to strings that are used by the guis
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
    mut datastore: Box<dyn DataRStore>,
    ui_fn: Box<dyn Fn(InternalPorts) -> Box<dyn Ui>>,
) -> (Box<dyn Ui>, Box<dyn traits::LocalUI>) {
    let (update_tx, update_rx) = mpsc::channel(32);
    let (intent_tx, intent_rx) = mpsc::channel(32);
    let (presenter_tx, presenter_rx) = mpsc::channel(4);

    let presenter = Presenter {
        update_rx,
        presenter_rx,
        data_updates: Box::into_pin(datastore.updates()),
        tasks: tasks::Tasks::new(),
    };

    let decoder = ActionDecoder {
        intent_tx,
        presenter_tx,
        datastore,
    };

    let ui = ui_fn(InternalPorts(decoder, presenter));
    let interface = Box::new(Interface::new(update_tx, intent_rx));

    (ui, interface)
}

#[derive(Debug, Clone)]
pub enum GuiUpdate {
    Exit,
    SearchResult(Vec<traits::SearchResult>),
    Data(DataUpdate),
    Error(String),
}

pub struct Presenter {
    update_rx: mpsc::Receiver<AppUpdate>,
    presenter_rx: mpsc::Receiver<ReqUpdate>,
    data_updates: Pin<Box<dyn Stream<Item = DataUpdate> + Send>>,
    tasks: tasks::Tasks,
}

impl Presenter {
    pub async fn update(&mut self) -> GuiUpdate {
        use futures::FutureExt;
        use futures_concurrency::future::Race;

        enum Res {
            App(AppUpdate),
            Req(ReqUpdate),
            Data(DataUpdate),
        }

        loop {
            let Self {
                update_rx,
                presenter_rx,
                tasks,
                data_updates,
            } = self;

            let res = {
                let next_update = update_rx
                    .recv()
                    .map(|msg| msg.expect("Interface should not drop before gui closes"))
                    .map(Res::App);
                let next_req = presenter_rx
                    .recv()
                    .map(|msg| msg.expect("ActionDecoder should not drop before Presenter"))
                    .map(Res::Req);
                let task_retval = tasks.next_retval().map(Res::App);
                let get_data = data_updates
                    .next()
                    .map(|msg| msg.expect("Data Updates should not drop before Presenter"))
                    .map(Res::Data);

                (next_update, next_req, task_retval, get_data).race().await
            };
            match res {
                Res::App(AppUpdate::Exit) => return GuiUpdate::Exit,
                Res::App(AppUpdate::Error(e)) => return GuiUpdate::Error(e),
                Res::App(AppUpdate::SearchResults(list)) => return GuiUpdate::SearchResult(list),
                Res::Req(ReqUpdate::Search(comms)) => tasks.add(comms),
                Res::Data(update) => return GuiUpdate::Data(update),
            }
        }
    }
}

pub struct ActionDecoder {
    intent_tx: mpsc::Sender<UserIntent>,
    /// used to send search request rx to Updater
    presenter_tx: mpsc::Sender<ReqUpdate>,
    datastore: Box<dyn DataRStore>,
}

// to do replace with functions instead of user action enum
impl ActionDecoder {
    pub fn key_press(&mut self, k: char) {
        if k == 'q' {
            self.intent_tx.try_send(UserIntent::Exit).unwrap();
            return;
        }

        tracing::warn!("unhandled key: {k:?}");
    }

    pub fn window_closed(&mut self) {
        self.intent_tx.try_send(UserIntent::Exit).unwrap();
    }

    #[instrument(skip(self))]
    pub fn search_enter(&mut self, query: String) {
        let (retval_tx, retval_rx) = oneshot::channel();
        self.presenter_tx
            .try_send(ReqUpdate::Search(retval_rx))
            .unwrap();
        let intent = UserIntent::FullSearch {
            query,
            awnser: retval_tx,
        };

        self.intent_tx.try_send(intent).unwrap();
    }
    pub fn searchleave(&mut self) {
        // self.presenter_tx.try_send(ReqUpdate::CancelSearch).unwrap();
        // return;
        todo!();
    }
    pub fn view_podcasts(&mut self) {
        self.datastore.sub_podcasts();
        return;
    }

    pub fn add_podcast(&self, podcast: SearchResult) {
        self.intent_tx
            .try_send(UserIntent::AddPodcast(podcast))
            .unwrap();
        return;
    }
}
