mod page;
mod database;
mod feed;
use page::Page;

use iced::{button, executor, Application, Command, Element, Column, Settings};

#[derive(Clone, Debug)]
pub enum Message {
    ToEpisodes(u64),
    PlayProgress(f32),
    Back,
    Pauze,
    Resume,
    Podcasts(page::podcasts::Message),
    Episodes(page::episodes::Message),
}

pub struct PlayBack {
    title: String,
    paused: bool,
    pos: f32,
    length: f32,
    playpauze: button::State,
}

pub struct App {
    current: Page,
    podcasts: page::Podcasts,
    episodes: page::Episodes,
    playing: Option<PlayBack>,
    back_button: button::State, //Should only be needed on desktop platforms
    db: sled::Db,
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (App, Command<Self::Message>) {
        let db = database::open().unwrap();
        (App {
            podcasts: page::Podcasts::new(&db), 
            episodes: page::Episodes::new(), 
            current: Page::Podcasts,
            playing: None, 
            back_button: button::State::new(),
            db, 
        }, Command::none())
    }
    fn title(&self) -> String {
        String::from("Podcasts")
    }
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        // dbg!(&message);
        match message {
            Message::Back => {
                self.current.back();
                Command::none()
            }
            Message::ToEpisodes(podcast_id) => {
                self.current = Page::Episodes;
                self.episodes.populate(podcast_id);
                Command::none()
            }
            Message::PlayProgress(p) => {
                self.playing.as_mut().unwrap().pos = p;
                Command::none()
            }
            Message::Pauze => Command::none(),
            Message::Resume => Command::none(),
            Message::Podcasts(m) => self.podcasts.update(m),
            Message::Episodes(m) => self.episodes.update(m),
        }
    }
    fn view(&mut self) -> Element<Self::Message> {
        dbg!("view");
        dbg!(&self.current);
        let content = match self.current {
            Page::Podcasts => self.podcasts.view(), // TODO load from a cache
            Page::Episodes => self.episodes.view(),
        };
        let column = Column::new();
        let column = column.push(content);
        let column = if let Some(playback) = &mut self.playing {
            column.push(page::draw_play_status(playback))
        } else {column};
        #[cfg(feature = "desktop")]
        let column = if self.current != Page::Podcasts {
            column.push(page::draw_back_button(&mut self.back_button))
        } else {column};
        
        iced::Container::new(column).into()
    }
}

pub fn main() -> iced::Result {
    let settings = build_settings();
    App::run(settings)
}

fn build_settings() -> Settings<()> {
    Settings {
        window: iced::window::Settings::default(),
        flags: (),
        default_font: None,
        default_text_size: 20,
        antialiasing: false,
    }
}
