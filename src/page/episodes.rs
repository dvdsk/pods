use iced::Element;
use iced::widget::scrollable::{self, Scrollable};

use crate::database::{PodcastKey, Progress};
use crate::widgets::style;
use crate::database::{Episode, PodcastDb, EpisodeKey};
use crate::download::{hash, FileType};
use crate::widgets::episode::{Collapsed, Expanded};
use std::collections::HashMap;


#[derive(Debug)]
pub struct CollapsedItem {
    pub title: String,
    age: String,
    duration: String,
    progress: Progress,
    pub file: Option<FileType>,
}

impl CollapsedItem {
    pub fn new(episode: Episode, episodes_on_disk: &HashMap<u64, FileType>) -> Self {
        let file = episodes_on_disk
            .get(&hash(&episode.title))
            .copied();

        Self {
            title: episode.title,
            age: episode.date.age_string(),
            duration: episode.duration.to_string(),
            progress: episode.progress,
            file, // none if no file on disk
        }
    }
    pub fn view(&self) -> Collapsed<crate::Message> {
        Collapsed::new(self.title.clone(), self.age.clone(), self.duration.clone())
    }
}

/// Episodes view
#[derive(Debug)]
pub struct Episodes {
    db: PodcastDb,
    list: Vec<CollapsedItem>,
    pub expanded: Option<usize>,
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
            expanded: None,
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
            self.list.push(CollapsedItem::new(info, &downloaded_episodes));
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
    pub fn view(&mut self, _theme: style::Theme) -> Element<crate::Message> {
        let mut scrollable = Scrollable::new(&mut self.scroll_state)
            .padding(10)
            .height(iced::Length::Fill);
        
        for (i, item) in self
            .list
            .iter_mut()
            .skip(self.scrolled_down)
            .take(Self::MAXSCROLLABLE)
            // .take(2)
            .enumerate()
        {
            let key = EpisodeKey::from_title(self.podcast_id.unwrap(), &item.title);
            let collapsed = item.view()
                .on_title(crate::Message::ToEpisodesDetails(i+self.scrolled_down))
                .on_plus(crate::Message::QueueEpisode(key));
            scrollable = if Some(i) == self.expanded {
                let key = EpisodeKey::from_title(self.podcast_id.unwrap(), &item.title);
                let description = self.db.get_episode_ext(key).unwrap().description;
                let expanded = Expanded::from_collapsed(collapsed, description);
                scrollable.push(expanded)
            } else {
                scrollable.push(collapsed)
            };
        }
        scrollable.into()
    }
}
