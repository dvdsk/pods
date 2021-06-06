pub mod episodes;
pub mod home;
pub mod podcasts;
mod errorpage;

use crate::Message;
use iced::{button, Button, Element, Length, Row, Text};
pub use episodes::Episodes;
pub use podcasts::Podcasts;
pub use home::Home;

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
    Home,
}

impl Page {
    pub fn back(&mut self) {
        *self = match &self {
            Self::Home => panic!("back not an option from home"),
            Self::Podcasts => Self::Home,
            Self::Episodes => Self::Podcasts,
        }
    }
}
