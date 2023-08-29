use std::collections::HashMap;

use async_trait::async_trait;
use color_eyre::eyre;
use tokio::sync::mpsc;

use tracing::instrument;
use traits::DataRStore;
use traits::DataSub;
use traits::DataUpdate;
use traits::EpisodeId;
use traits::PodcastId;
use traits::SearchResult;
pub use traits::{AppUpdate, UserIntent};

/* TODO: remove? <21-08-23, dvdsk> */
// mod tasks;

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
pub type UiBuilder = Box<dyn Fn(InternalPorts) -> Box<dyn Ui>>;

pub fn new(
    mut datastore: Box<dyn DataRStore>,
    ui_fn: Box<dyn Fn(InternalPorts) -> Box<dyn Ui>>,
) -> (Box<dyn Ui>, Box<dyn traits::LocalUI>) {
    let (update_tx, update_rx) = mpsc::channel(32);
    let (intent_tx, intent_rx) = mpsc::channel(32);
    let (data_tx, data_rx) = mpsc::channel(32);

    let registration = datastore.register(Box::new(data_tx), "presenter");
    let presenter = Presenter { update_rx, data_rx };

    let decoder = ActionDecoder {
        intent_tx,
        datastore,
        registration,
        subs: Subs::default(),
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
    data_rx: mpsc::Receiver<DataUpdate>,
}

impl Presenter {
    pub async fn update(&mut self) -> GuiUpdate {
        use futures::FutureExt;
        use futures_concurrency::future::Race;

        enum Res {
            App(AppUpdate),
            Data(DataUpdate),
        }

        loop {
            let Self {
                update_rx,
                data_rx: data_updates,
            } = self;

            let res = {
                let next_update = update_rx
                    .recv()
                    .map(|msg| msg.expect("Interface should not drop before gui closes"))
                    .map(Res::App);
                let get_data = data_updates
                    .recv()
                    .map(|msg| msg.expect("Data Updates should not drop before Presenter"))
                    .map(Res::Data);

                (next_update, get_data).race().await
            };
            match res {
                Res::App(AppUpdate::Exit) => return GuiUpdate::Exit,
                Res::App(AppUpdate::Error(e)) => return GuiUpdate::Error(e),
                Res::App(AppUpdate::SearchResults(list)) => return GuiUpdate::SearchResult(list),
                Res::Data(update) => return GuiUpdate::Data(update),
            }
        }
    }
}

#[derive(Default)]
struct Subs {
    podcast: Option<Box<dyn DataSub>>,
    episodes: HashMap<PodcastId, Box<dyn DataSub>>,
    episode_details: HashMap<EpisodeId, Box<dyn DataSub>>,
}

pub struct ActionDecoder {
    intent_tx: mpsc::Sender<UserIntent>,
    /// used to send search request rx to Updater
    datastore: Box<dyn DataRStore>,
    registration: traits::Registration,
    subs: Subs,
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
        let intent = UserIntent::FullSearch { query };
        self.intent_tx.try_send(intent).unwrap();
    }
    pub fn searchleave(&mut self) {
        // self.presenter_tx.try_send(ReqUpdate::CancelSearch).unwrap();
        // return;
        todo!();
    }
    pub fn view_podcasts(&mut self) {
        let sub = self.datastore.sub_podcasts(self.registration);
        self.subs.podcast = Some(sub);
    }

    pub fn view_episodes(&mut self, podcast: PodcastId) {
        let sub = self.datastore.sub_episodes(self.registration, podcast);
        self.subs.episodes.insert(podcast, sub);
    }

    pub fn view_episode_details(&mut self, episode: EpisodeId) {
        let sub = self.datastore.sub_episode_details(self.registration, episode);
        self.subs.episodes.insert(episode, sub);
    }

    pub fn add_podcast(&self, podcast: SearchResult) {
        self.intent_tx
            .try_send(UserIntent::AddPodcast(podcast))
            .unwrap();
    }
}
