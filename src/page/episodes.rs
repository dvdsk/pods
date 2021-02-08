use iced::Length;
use iced::{button, Button, Element, Text, HorizontalAlignment, Row};
use iced::widget::scrollable::{self, Scrollable};

use std::collections::HashMap;
use crate::database::{Episode, PodcastDb};
use crate::database::Progress;
use crate::database::{PodcastKey, EpisodeKey};

#[derive(Debug, Clone, Copy)]
pub enum FileType {
    Mp3,
}

impl FileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Mp3 => "mp3",
        }
    }
}

#[derive(Debug)]
struct ListItem {
    // either download or delete
    file_button: button::State,
    play_button: button::State,
    progress: Progress,
    file: Option<FileType>,
    title: String,
}

impl ListItem {
    fn from(episode: Episode, episodes_on_disk: &HashMap<u64, FileType>) -> Self {
        let title = episode.title.to_owned();
        let progress = episode.progress;
        let file = episodes_on_disk
            .get(&hash(&title))
            .copied();

        ListItem {
            file_button: button::State::new(),
            play_button: button::State::new(),
            progress,
            file, // is none if no file was found 
            title,
        }
    }
}

fn hash(string: &str) -> u64 {
    use std::hash::Hash;
    use std::hash::Hasher;
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    string.hash(&mut hasher);
    hasher.finish()
}

pub async fn scan_podcast_wrapper(list: Vec<Episode>, podcast: String) -> (HashMap<u64, FileType>, Vec<Episode>) {
    (scan_podcast_dir(podcast).await, list)
}

// TODO error handeling
pub async fn scan_podcast_dir(podcast: impl AsRef<str>) -> HashMap<u64, FileType> {
    use directories::UserDirs;
    use tokio::fs;

    let user_dirs = UserDirs::new()
        .expect("can not download if the user has no home directory");
    let mut dir = user_dirs.download_dir()
        .expect("need a download folder to be able to download")
        .to_owned();
    dir.push(env!("CARGO_BIN_NAME"));
    dir.push(podcast.as_ref());

    let mut set = HashMap::new();
    let entries = fs::read_dir(dir).await;
    if entries.is_err() {
        return set;
    }
    let mut entries = entries.unwrap();

    while let Some(entry) = entries.next_entry().await.unwrap() {
        let relative_path = entry.file_name();
        let relative_path = relative_path.to_str().unwrap();
        if let Some(name) = relative_path.strip_suffix(".mp3") 
            // .or_else(relative_path.strip_suffix(".other")
        { 
            log::trace!("found on disk episode: \"{}\"", name);
            set.insert(hash(name), FileType::Mp3);    
        }
    }
    set
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
    /// fill the view from a list of episodes
    pub fn populate(&mut self, episodes: Vec<Episode>, podcast_id: PodcastKey, downloaded_episodes: HashMap<u64, FileType>) {
        self.list.clear();
        self.podcast = Some(self.db.get_podcast(podcast_id).unwrap().title);
        self.podcast_id = Some(podcast_id);
        for info in episodes {
            self.list.push(ListItem::from(info, &downloaded_episodes));
        }
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
        for item in self.list
            .iter_mut()
            .skip(self.scrolled_down)
            .take(Self::MAXSCROLLABLE) {

            let podcast_id = *self.podcast_id.as_ref().unwrap();
            let key = EpisodeKey::from_title(podcast_id, &item.title);
            let mut row = Row::new();
            if let Some(file_type) = item.file {
                row = row.push(play_button(&mut item.play_button, key.clone(), file_type, item.title.clone(), item.progress.clone()));
                row = row.push(delete_button(&mut item.file_button, key.clone(), file_type));
            } else {
                row = row.push(stream_button(&mut item.play_button, key.clone(), item.title.clone()));
                row = row.push(download_button(&mut item.file_button, key.clone()));
            }
            scrollable = scrollable.push(row);
        }
        scrollable.into()
    }
}

fn play_button<'a>(state: &'a mut button::State, key: EpisodeKey, file_type: FileType, episode_name: String, progress: Progress)
    -> Button<'a, crate::Message> {
    let msg = crate::Message::Play(key, file_type, progress.into());
    Button::new(state, 
        Text::new(episode_name).horizontal_alignment(HorizontalAlignment::Left))
        .on_press(msg)
        .padding(12)
        .width(Length::FillPortion(4))
}

fn stream_button<'a>(state: &'a mut button::State, key: EpisodeKey, episode_name: String)
    -> Button<'a, crate::Message> {
    let msg = crate::Message::Stream(key);
    Button::new(state, 
        Text::new(episode_name).horizontal_alignment(HorizontalAlignment::Left))
        .on_press(msg)
        .padding(12)
        .width(Length::FillPortion(4))
}

fn download_button<'a>(state: &'a mut button::State, key: EpisodeKey)
    -> Button<'a, crate::Message> {
    let msg = crate::Message::Download(key);
    Button::new(state, 
        Text::new("dl").horizontal_alignment(HorizontalAlignment::Center))
        .on_press(msg)
        .padding(12)
        .width(Length::FillPortion(1))
}

fn delete_button<'a>(state: &'a mut button::State, key: EpisodeKey, file_type: FileType)
    -> Button<'a, crate::Message> {
    let msg = crate::Message::Remove(key, file_type);
    Button::new(state, 
        Text::new("rm").horizontal_alignment(HorizontalAlignment::Center))
        .on_press(msg)
        .padding(12)
        .width(Length::FillPortion(1))
}
