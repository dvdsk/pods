pub mod podcasts;
pub mod episodes;
mod errorpage;

pub use podcasts::Podcasts;
pub use episodes::Episodes;
pub use errorpage::errorpage;
use iced::Column;
use iced::{Element, button, Button, Text, Length, Row, Space};
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

pub fn draw_play_status(status: &mut super::PlayBack) -> Column<Message> {
    let podcast_len = 12.36; //FIXME dummy value
    let (button_text, button_action) = if status.paused {
        (Text::new("Pause"), Message::Pauze)
    } else {
        (Text::new("Resume"), Message::Resume)
    };

    let progress_bar = iced::ProgressBar::new(0.0..=status.length, status.pos);
    let mut controls = Row::new()
        .push(Space::with_width(Length::FillPortion(2)))
        .push(Button::new(&mut status.playpauze, button_text)
            .on_press(button_action)
            .width(Length::FillPortion(1)));
    
    Column::new()
        .push(progress_bar)
        .push(controls)
}
