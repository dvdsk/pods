use iced::Length;
use iced::{button, Button, Command, Element, Text, HorizontalAlignment};
use iced::widget::scrollable::{self, Scrollable};
use crate::Message;

pub type Home = Podcasts;
pub struct Podcasts {
    /// the podcasts title
    podcast_buttons: Vec<button::State>,
    podcast_names: Vec<String>,
    scroll_state: scrollable::State,
}

impl Podcasts {
    pub fn new() -> Self {
        let titles = ["99percentinvisible", "other_podcast"];

        let (podcast_names, podcast_buttons) = titles.iter()
            .map(|n| ( 
                n.to_owned().to_string(), 
                button::State::new() ))
            .unzip();
        let list = Podcasts {
            podcast_buttons,
            podcast_names,
            scroll_state: scrollable::State::new(),
        };
        list
    }
    pub fn update(&mut self) -> Command<Message> {
        dbg!("update");
        Command::none()
    }
    pub fn view(&mut self) -> Element<Message> {
        let mut scrollable = Scrollable::new(&mut self.scroll_state)
            .padding(10)
            .height(iced::Length::Fill);
        for (button, name) in self.podcast_buttons.iter_mut().zip(self.podcast_names.drain(..)) {
            scrollable = scrollable.push(
                Button::new(button, 
                    // Text::new(name)//.horizontal_alignment(HorizontalAlignment::Center)
                    Text::new("test").horizontal_alignment(HorizontalAlignment::Center)
                )
                .on_press(Message::ToEpisodes(name))
                .padding(12)
                .width(Length::Fill)
            )
        }
        // let scrollable = scrollable.push(Text::new("Hello world"));
        scrollable.into()
    }
}
