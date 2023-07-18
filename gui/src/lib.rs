mod home;
mod icon;
mod menu;
mod podcasts;

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use color_eyre::eyre;
use iced::{executor, Application, Subscription};
use presenter::{ActionDecoder, GuiUpdate, Presenter};

#[derive(Default, Clone, Debug)]
pub enum Page {
    #[default]
    Home,
    Podcasts,
    Search,
    Settings,
    Downloads,
    Playlists,
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
struct State {
    podcasts: podcasts::Podcasts,
    layout: Layout,
    rx: Arc<Mutex<Presenter>>,
    tx: ActionDecoder,
    should_exit: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToPage(Page),
    OpenMenu,
    CloseMenu,
    Gui(GuiUpdate),
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
                podcasts: podcasts::Podcasts::default(),
                rx: Arc::new(Mutex::new(rx)),
                tx,
                should_exit: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Panda Podcast")
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn update(&mut self, message: Self::Message) -> Command {
        match dbg!(message) {
            Message::Gui(GuiUpdate::Exit) => self.should_exit = true,
            Message::Gui(GuiUpdate::SearchResult(_)) => todo!(),
            Message::Gui(GuiUpdate::Data(_)) => todo!(),
            Message::ToPage(Page::Podcasts) => podcasts::load(&mut self.tx),
            Message::ToPage(page) => self.layout.to(page),
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
            Page::Search => todo!(),
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
