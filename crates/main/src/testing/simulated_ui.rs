use std::panic;
use std::sync::Arc;

use color_eyre::eyre;
use data::Data;
use presenter::{ActionDecoder, AppUpdate, GuiUpdate, Presenter, UiBuilder, UserIntent};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tokio::task::JoinError;
use tokio::time::timeout;
use traits::{async_trait, DataRStore, DataStore, DataUpdate, IndexSearcher, Settings};

use super::simulate_user::{Action, Condition, Steps, ViewableData};

pub struct SimulatedUI<'a> {
    rx: Presenter,
    tx: ActionDecoder,
    steps: Option<Steps<'a>>,
}

// only implemented so we can get a SimulatedUI from presenter::new
#[async_trait]
impl<'a> presenter::Ui for SimulatedUI<'a> {
    async fn run(&mut self) -> Result<(), eyre::Report> {
        unimplemented!()
    }
}

pub fn new_simulated_ui(interface: presenter::InternalPorts) -> Box<dyn presenter::Ui> {
    let presenter::InternalPorts(tx, rx) = interface;
    Box::new(SimulatedUI {
        rx,
        tx,
        steps: None,
    })
}

use tokio::sync::mpsc;
struct SimulatedUIPorts {
    update_tx: mpsc::Sender<AppUpdate>,
    intent_reciever: traits::LocalIntentReciever,
}

pub struct State;
#[derive(Debug)]
pub enum Error {
    TimeoutError { step: usize },
}

impl<'a> SimulatedUI<'a> {
    fn new(
        steps: Steps<'a>,
        data: Box<dyn DataRStore>,
    ) -> (SimulatedUI<'a>, Box<dyn traits::LocalUI>) {
        let ui_fn = Box::new(new_simulated_ui) as UiBuilder;
        let (ui, ports) = presenter::new(data, ui_fn);
        // presenter returns the ui as a trait obj. We want to
        // use presenter so but need a SimulatedUI, therefore we
        // turn it into a concrete type again
        let mut ui: Box<SimulatedUI> =
            unsafe { Box::from_raw(Box::into_raw(ui) as *mut SimulatedUI) };
        ui.steps = Some(steps);
        (*ui, ports)
    }

    async fn run(&mut self) -> Result<State, Error> {
        use ViewableData::{Podcast, PodcastList};
        let duration = self.steps.as_ref().map(|s| s.timeout).unwrap();
        let list = &mut self.steps.as_mut().unwrap().list;
        for (i, (condition, action)) in list.iter_mut().enumerate() {
            match condition {
                Condition::None => (),
                Condition::DataUpdate(update) => {
                    timeout(duration, data_update(&mut self.rx, update.clone()))
                        .await
                        .map_err(|_| Error::TimeoutError { step: i })?;
                }
                Condition::DataUpdateAndFnMut { update, func } => {
                    timeout(
                        duration,
                        data_update_fnmut(&mut self.rx, update.clone(), func),
                    )
                    .await
                    .map_err(|_| Error::TimeoutError { step: i })?;
                }
                other => todo!("{other:?}"),
            }
            match action {
                Action::Intent(UserIntent::AddPodcast(p)) => self.tx.add_podcast(p.clone()),
                Action::Stop => break,
                Action::View(PodcastList) => self.tx.view_podcasts(),
                Action::View(Podcast { podcast_id }) => self.tx.view_episodes(*podcast_id),
                other => todo!("{other:?}"),
            }
        }
        Ok(State)
    }
}

async fn data_update_fnmut(
    rx: &mut Presenter,
    update: traits::DataUpdateVariant,
    func: &mut dyn FnMut(&DataUpdate) -> bool,
) {
    loop {
        let GuiUpdate::Data(got) = rx.update().await else {
            continue;
        };
        if got.variant() == update {
            if func(&got) {
                break;
            }
        }
    }
}

async fn data_update(rx: &mut Presenter, update: traits::DataUpdateVariant) {
    loop {
        let GuiUpdate::Data(got) = rx.update().await else {
            continue;
        };
        if got.variant() == update {
            break;
        }
    }
}

impl traits::LocalUI for SimulatedUIPorts {
    fn ports(&mut self) -> (&mut dyn traits::Updater, &mut dyn traits::IntentReciever) {
        (&mut self.update_tx, &mut self.intent_reciever)
    }
}

enum Res {
    Data(Result<(), JoinError>),
    Media(Box<dyn std::any::Any + Send + 'static>),
    App(Result<(), JoinError>),
    UI(Result<State, Error>),
}
use futures::FutureExt;
use futures_concurrency::future::Race;

async fn run_inner<'a>(steps: Steps<'a>) -> Result<State, Error> {
    let (mut data, data_maintain) = Data::new();
    let server_config = data.settings_mut().server().get_value();

    let (mut ui, ui_port) = SimulatedUI::new(steps, data.reader());

    let remote = Box::new(remote_ui::new(server_config));
    let searcher = Arc::new(Mutex::new(search::new())) as Arc<Mutex<dyn IndexSearcher>>;

    let data = Box::new(data) as Box<dyn DataStore>;
    let (media, mut media_handle) = media::Media::new();
    let media = Box::new(media);
    let player = Box::new(player::Player::new());
    let feed = Box::new(feed::Feed::new());

    let app = tokio::task::spawn(panda::app(
        data,
        Some(ui_port),
        remote,
        searcher,
        media,
        player,
        feed,
    ))
    .map(Res::App);
    let ui = ui.run().map(Res::UI);
    let data_maintain = data_maintain.map(Res::Data);
    let media_handle = media_handle.errors().map(Res::Media);

    match (app, ui, data_maintain, media_handle).race().await {
        Res::Data(Err(e)) | Res::App(Err(e)) => panic::resume_unwind(e.into_panic()),
        Res::Media(e) => panic::resume_unwind(e),
        Res::UI(Err(e)) => panic!("UI ran into error: {e:?}"),
        Res::UI(Ok(state)) => return Ok(state),
        _ => unreachable!(),
    }
}

impl<'a> Steps<'a> {
    pub fn run(self) -> Result<State, Error> {
        let rt = Runtime::new().unwrap();
        rt.block_on(async { run_inner(self).await })
    }
}
