use iced::widget::scrollable::{self, Scrollable};
use iced::Length;
use iced::{button, Button, Element, HorizontalAlignment, Row, Text};

use crate::database::Progress;
use crate::database::{Episode, PodcastDb};
use crate::database::{EpisodeKey, PodcastKey};
use crate::download::{hash, FileType};
use std::collections::HashMap;

#[derive(Debug)]
struct ListItem {
    // either download or delete
    file_button: button::State,
    play_button: button::State,
    progress: Progress,
    file: Option<FileType>,
    title: String,
    date: String,
}

impl ListItem {
    fn from(episode: Episode, episodes_on_disk: &HashMap<u64, FileType>) -> Self {
        let title = episode.title.to_owned();
        let progress = episode.progress;
        let file = episodes_on_disk.get(&hash(&title)).copied();

        let date = episode.date.format();

        ListItem {
            file_button: button::State::new(),
            play_button: button::State::new(),
            progress,
            file, // is none if no file was found
            title,
            date,
        }
    }
}

/// Episodes view
#[derive(Debug)]
pub struct Episodes {
    db: PodcastDb,
    list: Vec<ListItem>,
    scroll_state: scrollable::State,
    pub podcast: Option<String>,
    podcast_id: Option<PodcastKey>,
    // number of rows we scrolled down
    scrolled_down: usize,
}

impl Episodes {
    const MAXSCROLLABLE: usize = 10;
    pub fn from_db(db: PodcastDb) -> Self {
        Self {
            db,
            list: Vec::new(),
            scroll_state: scrollable::State::new(),
            podcast: None,
            podcast_id: None,
            scrolled_down: 0,
        }
    }
    pub fn down(&mut self) {
        self.scrolled_down += Self::MAXSCROLLABLE;
        self.scrolled_down = self.scrolled_down.min(self.list.len());
    }
    pub fn up(&mut self) {
        self.scrolled_down = self.scrolled_down.saturating_sub(Self::MAXSCROLLABLE);
    }
    pub fn repopulate(&mut self, downloaded_episodes: HashMap<u64, FileType>) {
        let podcast_id = self.podcast_id.unwrap();
        self.list.clear();
        self.podcast = Some(self.db.get_podcast(podcast_id).unwrap().title);

        let mut episodes = self.db.get_episodes(podcast_id).unwrap();
        episodes.sort_unstable_by_key(|e| *e.date.inner());
        episodes.reverse();
        for info in episodes {
            self.list.push(ListItem::from(info, &downloaded_episodes));
        }
    }
    /// fill the view from a list of episodes
    pub fn populate(
        &mut self,
        podcast_id: PodcastKey,
        downloaded_episodes: HashMap<u64, FileType>,
    ) {
        self.podcast_id = Some(podcast_id);
        self.repopulate(downloaded_episodes);
    }
    pub fn update_downloaded(&mut self, downloaded_episodes: HashMap<u64, FileType>) {
        for item in &mut self.list {
            let file = downloaded_episodes.get(&hash(&item.title)).copied();
            item.file = file;
        }
    }
    pub fn view(&mut self) -> Element<crate::Message> {
        let mut scrollable = Scrollable::new(&mut self.scroll_state)
            .padding(10)
            .height(iced::Length::Fill);
        for item in self
            .list
            .iter_mut()
            .skip(self.scrolled_down)
            .take(Self::MAXSCROLLABLE)
        {
            let podcast_id = *self.podcast_id.as_ref().unwrap();
            let key = EpisodeKey::from_title(podcast_id, &item.title);
            let mut row = Row::new();
            if let Some(file_type) = item.file {
                row = row.push(play_button(
                    &mut item.play_button,
                    key,
                    file_type,
                    item.title.clone(),
                    item.progress,
                ));
                row = row.push(delete_button(&mut item.file_button, key, file_type));
            } else {
                row = row.push(stream_button(
                    &mut item.play_button,
                    key,
                    item.title.clone(),
                ));
                row = row.push(download_button(&mut item.file_button, key));
            }
            scrollable = scrollable.push(row);
        }
        scrollable.into()
    }
}

fn play_button(
    state: &mut button::State,
    key: EpisodeKey,
    file_type: FileType,
    episode_name: String,
    progress: Progress,
) -> Button<crate::Message> {
    let msg = crate::Message::Play(key, file_type, progress.into());
    Button::new(
        state,
        Text::new(episode_name).horizontal_alignment(HorizontalAlignment::Left),
    )
    .on_press(msg)
    .padding(12)
    .width(Length::FillPortion(4))
}

fn stream_button(
    state: &mut button::State,
    key: EpisodeKey,
    episode_name: String,
) -> Button<crate::Message> {
    let msg = crate::Message::Stream(key);
    Button::new(
        state,
        Text::new(episode_name).horizontal_alignment(HorizontalAlignment::Left),
    )
    .on_press(msg)
    .padding(12)
    .width(Length::FillPortion(4))
}

fn download_button(state: &mut button::State, key: EpisodeKey) -> Button<crate::Message> {
    let msg = crate::Message::Download(key);
    Button::new(
        state,
        Text::new("dl").horizontal_alignment(HorizontalAlignment::Center),
    )
    .on_press(msg)
    .padding(12)
    .width(Length::FillPortion(1))
}

fn delete_button(
    state: &mut button::State,
    key: EpisodeKey,
    file_type: FileType,
) -> Button<crate::Message> {
    let msg = crate::Message::Remove(key, file_type);
    Button::new(
        state,
        Text::new("rm").horizontal_alignment(HorizontalAlignment::Center),
    )
    .on_press(msg)
    .padding(12)
    .width(Length::FillPortion(1))
}
