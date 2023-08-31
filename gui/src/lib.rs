mod home;
mod icon;
mod menu;
mod podcast;
mod podcasts;

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use traits::{DataUpdateVariant, Episode, EpisodeDetails, PodcastId};

use color_eyre::eyre;
use iced::{executor, window, Application, Subscription};
use presenter::{ActionDecoder, GuiUpdate, Presenter};

#[derive(Default, Clone, Debug)]
pub enum Page {
    #[default]
    Home,
    Podcasts,
    Podcast {
        id: traits::PodcastId,
        details: Option<traits::EpisodeId>,
    },
    AddPodcast,
    Settings,
    Downloads,
    Playlists,
}

impl Page {
    pub(crate) fn load(&self, state: &mut crate::State) {
        match self {
            Page::Podcasts => podcasts::load(state),
            Page::Podcast { id, details } => podcast::load(state, *id, *details),

            _ => state.layout.to(self.clone()),
        }
    }
}

#[derive(Default)]
struct Layout {
    page: Page,
    in_menu: bool, // default is false
}

impl Layout {
    fn to(&mut self, page: Page) {
        self.page = page;
        self.in_menu = false;
    }
}

#[derive(Debug)]
struct Loading {
    pub needed_data: HashSet<DataUpdateVariant>,
    pub page: Page,
}

impl Loading {
    fn new<const N: usize>(page: Page, needed: [DataUpdateVariant; N]) -> Self {
        Self {
            needed_data: HashSet::from(needed),
            page,
        }
    }

    /// returns true if one of the loading requirements is
    /// the Episodes list for the podcast with podcast_id
    fn episodes_for(&self, podcast_id: PodcastId) -> bool {
        self.needed_data
            .contains(&DataUpdateVariant::Episodes { podcast_id })
    }
}

pub(crate) struct Podcast {
    name: String,
    id: PodcastId,
    episodes: Vec<Episode>,
    details: Option<EpisodeDetails>,
}

struct State {
    podcasts: podcasts::Podcasts,
    podcast: Option<Podcast>,
    search: podcasts::add::Search,
    layout: Layout,
    loading: Option<Loading>,
    rx: Arc<Mutex<Presenter>>,
    tx: ActionDecoder,
}
impl State {
    fn handle_data(&mut self, data: traits::DataUpdate) {
        use traits::DataUpdate::*;
        match data {
            Podcasts { podcasts } => self.podcasts = podcasts,
            Episodes { list, podcast_id } => {
                if let Some(loading) = &self.loading {
                    if loading.episodes_for(podcast_id) {
                        self.podcast.as_mut().unwrap().episodes = list;
                        return;
                    }
                }
                if let Some(podcast) = &mut self.podcast {
                    if podcast.id == podcast_id {
                        podcast.episodes = list
                    }
                }
            }
            Placeholder => panic!("Placeholder should never be used"),
            EpisodeDetails { details } => {
                if let Some(podcast) = &mut self.podcast {
                    if podcast.id == details.id {
                        podcast.details = Some(details);
                    }
                }
            }
            Missing { variant } => todo!("missing data for {variant:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ToPage(Page),
    OpenMenu,
    CloseMenu,
    Gui(GuiUpdate),
    SearchUpdate(String),
    SearchDetails(podcasts::add::ResultIdx),
    AddPodcast(podcasts::add::ResultIdx),
    SearchDetailsClose,
}

#[derive(Debug)]
enum Event {}

type Command = iced::Command<Message>;
impl Application for State {
    type Message = Message;
    type Executor = executor::Default;
    type Theme = iced::Theme;
    type Flags = (Presenter, ActionDecoder);

    fn new((rx, tx): Self::Flags) -> (State, Command) {
        (
            State {
                layout: Layout::default(),
                loading: None,
                podcasts: podcasts::Podcasts::default(),
                podcast: None,
                rx: Arc::new(Mutex::new(rx)),
                tx,
                search: podcasts::add::Search::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Panda Podcast")
    }

    fn update(&mut self, message: Self::Message) -> Command {
        match message {
            Message::Gui(GuiUpdate::Exit) => return window::close(),
            Message::Gui(GuiUpdate::Error(e)) => panic!("Error: {e:?}"),
            Message::Gui(GuiUpdate::SearchResult(results)) => self.search.update_results(results),
            Message::Gui(GuiUpdate::Data(data)) => {
                /* TODO: can we move this to presenter? <dvdsk noreply@davidsk.dev> */
                let variant = data.variant();
                self.handle_data(data);
                if let Some(Loading { needed_data, page }) = &mut self.loading {
                    needed_data.remove(&variant);
                    if needed_data.is_empty() {
                        self.layout.to(page.clone());
                    }
                }
                dbg!(&self.loading);
            }
            Message::SearchUpdate(query) => self.search.update_query(query, &mut self.tx),
            Message::SearchDetails(idx) => self.search.open_details(idx),
            Message::SearchDetailsClose => self.search.close_details(),
            Message::AddPodcast(idx) => {
                self.search.add_podcast(idx, &mut self.tx);
                self.loading = Some(Loading {
                    needed_data: HashSet::from([DataUpdateVariant::Podcasts]),
                    page: Page::Podcasts,
                });
            }
            Message::ToPage(page) => page.load(self),
            Message::CloseMenu => self.layout.in_menu = false,
            Message::OpenMenu => self.layout.in_menu = true,
        }
        Command::none()
    }

    fn view(&self) -> iced::Element<Message> {
        let column = menu::view_bar(self.layout.in_menu);
        if self.layout.in_menu {
            return menu::view(column).into();
        }
        match self.layout.page {
            Page::Home => home::view(column),
            Page::Podcasts => podcasts::view(column, &self.podcasts),
            Page::Podcast { .. } => podcast::view(column, self.podcast.as_ref().unwrap()),
            Page::AddPodcast => self.search.view(column),
            Page::Settings => todo!(),
            Page::Downloads => todo!(),
            Page::Playlists => todo!(),
        }
        .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        update_sub(self.rx.clone())
    }
}

fn update_sub(rx: Arc<Mutex<Presenter>>) -> iced::Subscription<Message> {
    struct GetUpdates;
    let id = std::any::TypeId::of::<GetUpdates>();
    iced::subscription::unfold(id, rx, move |rx| async {
        let msg = {
            let mut presenter = rx.try_lock().expect("locking should always succeed");
            let update = presenter.update().await;
            let msg = Message::Gui(update);
            msg
        };
        (Some(msg), rx)
    })
}

pub struct IcedGui {
    rx: Option<Presenter>,
    tx: Option<ActionDecoder>,
}

pub fn new(interface: presenter::InternalPorts) -> Box<dyn presenter::Ui> {
    let presenter::InternalPorts(tx, rx) = interface;
    Box::new(IcedGui {
        rx: Some(rx),
        tx: Some(tx),
    })
}

#[async_trait]
impl presenter::Ui for IcedGui {
    async fn run(&mut self) -> Result<(), eyre::Report> {
        let settings =
            iced::Settings::with_flags((self.rx.take().unwrap(), self.tx.take().unwrap()));
        tokio::task::block_in_place(|| State::run(settings))?;
        Ok(())
    }
}
