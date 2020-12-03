use iced::Length;
use iced::{button, Button, Element, Text, HorizontalAlignment, Row};
use iced::widget::scrollable::{self, Scrollable};

use crate::database::{self, podcasts::EpisodeList};

#[derive(Debug)]
pub struct Episodes {
    db: database::Episodes,
    episode_buttons: Vec<(button::State, button::State)>,
    episode_names: Vec<String>,
    scroll_state: scrollable::State,
    title: String,
}

impl Episodes {
    pub fn from_db(db: &database::Episodes) -> Self {
        Self {
            db: db.clone(),
            episode_buttons: Vec::new(),
            episode_names: Vec::new(),
            scroll_state: scrollable::State::new(),
            title: String::new(),
        }
    }
    pub fn populate(&mut self, episodes: EpisodeList) {
        self.episode_names.clear();
        self.episode_buttons.clear();
        for info in episodes {
            self.episode_names.push(info.title); 
            self.episode_buttons.push((button::State::new(),button::State::new()));
        }
    }
    pub fn view(&mut self) -> Element<crate::Message> {
        let mut scrollable = Scrollable::new(&mut self.scroll_state)
            .padding(10)
            .height(iced::Length::Fill);
        for ((b1,b2), name) in self.episode_buttons.iter_mut().zip(self.episode_names.iter()) {
            scrollable = scrollable.push(Row::new()
                .push(play_button(b1, &self.title, &name))
                .push(download_button(b2, &self.title, &name))
            );
        }
        scrollable.into()
    }
}

fn play_button<'a>(state: &'a mut button::State, podcast_name: &str, episode_name: &str)
    -> Button<'a, crate::Message> {
    let key = database::episodes::Key::from((podcast_name, episode_name));
    let msg = crate::Message::Play(key);
    Button::new(state, 
        Text::new(episode_name.to_owned()).horizontal_alignment(HorizontalAlignment::Left))
        .on_press(msg)
        .padding(12)
        .width(Length::FillPortion(4))
}

fn download_button<'a>(state: &'a mut button::State, podcast_name: &str, episode_name: &str)
    -> Button<'a, crate::Message> {
    let key = (podcast_name.to_owned(), episode_name.to_owned());
    let msg = crate::Message::Download(key);
    Button::new(state, 
        Text::new("dl").horizontal_alignment(HorizontalAlignment::Center))
        .on_press(msg)
        .padding(12)
        .width(Length::FillPortion(1))
        // .height(Length::Fill)
}
