mod page;
use page::Page;

use iced::{executor, Application, Command, Element, Column, Settings};

#[derive(Clone, Debug)]
pub enum Message {
    ToEpisodes(String),
}

pub struct App {
    current: Page,
    home: page::Home,
    episodes: Option<page::Episodes>,
    // play_pause:
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(flags: Self::Flags) -> (App, Command<Self::Message>) {
        (App {
            home: page::Home::new(), 
            episodes: None, 
            current: Page::Home,
        }, Command::none())
    }
    fn title(&self) -> String {
        String::from("A test title")
    }
    fn update(&mut self, _message: Self::Message) -> Command<Self::Message> {
        match self.current {
            Page::Home => self.home.update(),
            Page::Episodes => self.episodes.as_mut().unwrap().update(),
        }
    }
    fn view(&mut self) -> Element<Self::Message> {
        let content = match self.current {
            Page::Home => self.home.view(),
            Page::Episodes => self.episodes.as_mut()
                .map(|e| e.view())
                .unwrap_or(page::errorpage()),
        };
        let column: Element<_> = Column::new()
            .push(content)
            .into();
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

// #[tokio::main]
// async fn main() {

//     let example = include_bytes!("99percentinvisible");
//     let channel = rss::Channel::read_from(&example[..]).unwrap();
//     for title in channel.items().iter().filter_map(|x| x.title()) {
//         println!("{}", title);
//     }
    
//     println!("Hello, world!");
// }
