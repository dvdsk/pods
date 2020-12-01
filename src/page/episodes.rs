use iced::Length;
use iced::{button, Button, Command, Element, Text, HorizontalAlignment};
use iced::widget::scrollable::{self, Scrollable};

use crate::database::podcasts::EpisodeList;

#[derive(Debug, Default)]
pub struct Episodes {
    /// the episodes title
    episode_buttons: Vec<button::State>,
    episode_names: Vec<String>,
    scroll_state: scrollable::State,
    title: String,
}

impl Episodes {
    pub fn new() -> Self {
        Episodes::default()
    }
    pub fn populate(&mut self, episodes: EpisodeList) {
        self.episode_names.clear();
        self.episode_buttons.clear();
        for info in episodes {
            self.episode_names.push(info.title); 
            self.episode_buttons.push(button::State::new());
        }
    }
    pub fn view(&mut self) -> Element<crate::Message> {
        let mut scrollable = Scrollable::new(&mut self.scroll_state)
            .padding(10)
            .height(iced::Length::Fill);
        for (button, name) in self.episode_buttons.iter_mut().zip(self.episode_names.iter()) {
            scrollable = scrollable.push(
                Button::new(button, 
                    Text::new(name.to_owned()).horizontal_alignment(HorizontalAlignment::Center)
                )
                //Todo replace content of ToEpisode with some key
                .on_press(crate::Message::Play(name.to_owned()))
                .padding(12)
                .width(Length::Fill)
            )
        }
        scrollable.into()
    }
}


