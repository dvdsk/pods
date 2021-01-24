use iced::Length;
use iced::{button, Button, Element, Text, HorizontalAlignment, Row, Column};
use iced::widget::scrollable::{self, Scrollable};

use crate::database::{self, podcasts::EpisodeList};

/// Episodes view
#[derive(Debug)]
pub struct Episodes {
    db: database::Episodes,
    episode_buttons: Vec<(button::State, button::State)>,
    episode_names: Vec<String>,
    scroll_state: scrollable::State,
    podcast_id: u64,
    // number of rows we scrolled down
    scrolled_down: usize,
}

impl Episodes {
    pub fn from_db(db: &database::Episodes) -> Self {
        Self {
            db: db.clone(),
            episode_buttons: Vec::new(),
            episode_names: Vec::new(),
            scroll_state: scrollable::State::new(),
            podcast_id: 0,
            scrolled_down: 0,
        }
    }
    pub fn down(&mut self) {
        self.scrolled_down += 10;
        self.scrolled_down = self.scrolled_down.min(self.episode_buttons.len());
    }
    pub fn up(&mut self) {
        self.scrolled_down -= 10;
        self.scrolled_down = self.scrolled_down.max(0);
    }
    /// fill the view from a list of episodes
    pub fn populate(&mut self, podcast_id: u64, episodes: EpisodeList) {
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
        for ((b1,b2), name) in self.episode_buttons
            .iter_mut()
            .zip(self.episode_names.iter())
            .skip(self.scrolled_down)
            .take(15) {

            scrollable = scrollable.push(Row::new()
                .push(play_button(b1, self.podcast_id, &name))
                .push(download_button(b2, self.podcast_id, &name))
            );
        }
        scrollable.into()
    }
}

fn play_button<'a>(state: &'a mut button::State, podcast_id: u64, episode_name: &str)
    -> Button<'a, crate::Message> {
    let key = database::episodes::Key::from((podcast_id, episode_name));
    let msg = crate::Message::Play(key);
    Button::new(state, 
        Text::new(episode_name.to_owned()).horizontal_alignment(HorizontalAlignment::Left))
        .on_press(msg)
        .padding(12)
        .width(Length::FillPortion(4))
}

fn download_button<'a>(state: &'a mut button::State, podcast_id: u64, episode_name: &str)
    -> Button<'a, crate::Message> {
    let key = database::episodes::Key::from((podcast_id, episode_name));
    let msg = crate::Message::Download(key);
    Button::new(state, 
        Text::new("dl").horizontal_alignment(HorizontalAlignment::Center))
        .on_press(msg)
        .padding(12)
        .width(Length::FillPortion(1))
        // .height(Length::Fill)
}
