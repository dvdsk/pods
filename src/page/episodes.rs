use iced::{Command, button, Element, Text};
use crate::Message;

struct EpisodeButton {
    name: String,
    button: button::State,
}

pub struct Episodes {
    /// the podcasts title
    title: String,
    episode_buttons: Vec<EpisodeButton>,
}

impl Episodes {
    fn new(channel: & rss::Channel) -> Self {
        
        let buttons = channel.items()
            .iter().filter_map(|x| x.title())
            .map(|n| EpisodeButton {name: n.to_owned(), button: button::State::new()})
            .collect();
        let list = Episodes {
            title: channel.title().to_owned(),
            episode_buttons: buttons,
        };
        list
    }
    pub fn update(&mut self) -> Command<Message> {
        Command::none()
    }
    pub fn view(&mut self) -> Element<Message> {
        Text::new("Hello world").into()
    }
}


