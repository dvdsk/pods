pub mod episodes;
mod errorpage;
pub mod podcasts;

use crate::Message;
pub use episodes::Episodes;
use iced::{button, Button, Element, Length, Row, Text};
pub use podcasts::Podcasts;

#[derive(Default)]
pub struct Controls {
    pub back: button::State,
    pub up: button::State,
    pub down: button::State,
}

impl Controls {
    pub fn view(&mut self) -> Element<crate::Message> {
        let row = Row::new();
        let row = row.push(
            Button::new(&mut self.back, Text::new("back".to_owned()))
                .on_press(Message::Back)
                .width(Length::Fill),
        );
        let row = row.push(
            Button::new(&mut self.up, Text::new("up".to_owned()))
                .on_press(Message::Up)
                .width(Length::Fill),
        );
        let row = row.push(
            Button::new(&mut self.down, Text::new("down".to_owned()))
                .on_press(Message::Down)
                .width(Length::Fill),
        );
        row.into()
    }
}

#[derive(Debug, PartialEq)]
pub enum Page {
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
