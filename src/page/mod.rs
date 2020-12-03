pub mod podcasts;
pub mod episodes;
mod errorpage;

pub use podcasts::Podcasts;
pub use episodes::Episodes;
use iced::{button, Button, Text, Length};
use crate::Message;

#[derive(Debug, PartialEq)]
pub enum Page{
    Podcasts,
    Episodes,
}

impl Page {
    pub fn back(&mut self) {
        *self = match &self {
            Self::Podcasts => Self::Podcasts,
            Self::Episodes => Self::Podcasts,
        }
    }
}

pub fn draw_back_button(state: &mut button::State) -> Button<Message> {
    Button::new(state, Text::new("back".to_owned()))
        .on_press(Message::Back)
        .width(Length::Fill)
}
