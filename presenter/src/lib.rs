use std::future;

use async_trait::async_trait;
use color_eyre::eyre;
use tokio::sync::{mpsc, oneshot};

use traits::ReqUpdate;
pub use traits::{AppUpdate, UserIntent};


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
    ui_fn: Box<dyn Fn(InternalPorts) -> Box<dyn Ui>>,
) -> (Box<dyn Ui>, Box<dyn traits::LocalUI>) {
    let (update_tx, update_rx) = mpsc::channel(32);
    let (intent_tx, intent_rx) = mpsc::channel(32);
    let (presenter_tx, presenter_rx) = mpsc::channel(4);
    let decoder = ActionDecoder {
        intent_tx,
        presenter_tx,
    };
    let presenter = Presenter {
        update_rx,
        presenter_rx,
        search: Search(None),
    };
    let ui = ui_fn(InternalPorts(decoder, presenter));
    let interface = Box::new(Interface::new(update_tx, intent_rx));

    (ui, interface)
}

#[derive(Debug, Clone)]
pub enum GuiUpdate {
    Exit,
    SearchResult(Vec<traits::SearchResult>),
}

pub struct Presenter {
    update_rx: mpsc::Receiver<AppUpdate>,
    presenter_rx: mpsc::Receiver<ReqUpdate>,
    search: Search,
}

struct Search(Option<oneshot::Receiver<Vec<traits::SearchResult>>>);

impl Search {
    async fn wait(&mut self) -> Vec<traits::SearchResult> {
        match self.0.as_mut() {
            Some(s) => s.await.expect("Search crashed"),
            None => future::pending().await,
        }
    }
    fn cancel(&mut self) {
        self.0 = None
    }

    fn start(&mut self, search: oneshot::Receiver<Vec<traits::SearchResult>>) {
        self.0 = Some(search)
    }
}

impl Presenter {
    pub async fn update(&mut self) -> GuiUpdate {
        use futures::FutureExt;
        use futures_concurrency::future::Race;

        enum Res {
            App(AppUpdate),
            Req(ReqUpdate),
            Search(Vec<traits::SearchResult>),
        }

        loop {
            let Self {
                update_rx,
                presenter_rx,
                search,
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
                let do_search = search.wait().map(Res::Search);

                (next_update, next_req, do_search).race().await
            };
            match res {
                Res::App(AppUpdate::Exit) => return GuiUpdate::Exit,
                Res::Req(ReqUpdate::CancelSearch) => search.cancel(),
                Res::Req(ReqUpdate::Search(comms)) => search.start(comms),
                Res::Search(list) => return GuiUpdate::SearchResult(list),
            }
        }
    }
}

#[derive(Debug)]
pub enum UserAction {
    KeyPress(char),
    WindowClosed,
    SearchEnter(String),
    SearchLeave,
}

pub struct ActionDecoder {
    intent_tx: mpsc::Sender<UserIntent>,
    /// used to send search request rx to Updater
    presenter_tx: mpsc::Sender<ReqUpdate>,
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
            UserAction::SearchEnter(query) => {
                let (tx, rx) = oneshot::channel();
                self.presenter_tx.send(ReqUpdate::Search(rx)).await.unwrap();
                UserIntent::FullSearch { query, awnser: tx }
            }
            UserAction::SearchLeave => {
                self.presenter_tx
                    .send(ReqUpdate::CancelSearch)
                    .await
                    .unwrap();
                return;
            }
        };

        self.intent_tx.send(intent).await.unwrap();
    }
}
