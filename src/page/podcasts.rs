use iced::Length;
use iced::{button, Button, Column, Command, Element, HorizontalAlignment, Text};
use iced::{scrollable, Scrollable};
use iced::{text_input, TextInput};
use itertools::izip;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::{self, PodcastDb, PodcastKey};
use crate::{feed, Message};
use crate::iced_wrapped;

#[derive(Default)]
pub struct Search {
    input: text_input::State,
    input_value: String,
    search: Arc<Mutex<feed::Search>>,
}

impl Search {
    pub fn do_search(&mut self, ignore_budget: bool) -> Command<crate::Message> {
        // always do a web search if a search was submitted
        let term = self.input_value.clone();
        let search = self.search.clone();
        Command::perform(
            async move { search.lock().await.search(term, ignore_budget).await },
            Message::SearchResults,
        )
    }
    pub fn input_changed(&mut self, pod_db: PodcastDb, input: String) -> Command<crate::Message> {
        self.input_value = input;
        if feed::valid_url(&self.input_value) {
            let url = self.input_value.clone();
            iced_wrapped::add_podcast(&pod_db, url)
        } else if self.input_value.len() > 4 {
            self.do_search(false)
        } else {
            Command::none()
        }
    }
    pub fn view(&mut self) -> TextInput<crate::Message> {
        TextInput::new(
            &mut self.input,
            "Add podcast url",
            &self.input_value,
            Message::SearchInputChanged,
        )
        .width(Length::Fill)
        .on_submit(Message::SearchSubmit)
    }
    pub fn reset(&mut self) {
        self.input_value.clear();
    }
}

#[derive(Default)]
pub struct List {
    podcast_buttons: Vec<(PodcastKey, button::State)>,
    podcast_names: Vec<String>,
    feedres_buttons: Vec<button::State>,
    feedres_info: Vec<feed::SearchResult>,
    scroll_state: scrollable::State,
    scrolled_down: usize,
}

fn feedres_button(button: &mut button::State, res: feed::SearchResult) -> Button<crate::Message> {
    Button::new(
        button,
        Text::new(res.title).horizontal_alignment(HorizontalAlignment::Center),
    )
    //Todo replace content of ToEpisode with some key
    .on_press(crate::Message::AddPodcast(res.url))
    .padding(12)
    .width(Length::Fill)
}
fn podcast_button(
    button: &mut button::State,
    text: String,
    id: PodcastKey,
) -> Button<crate::Message> {
    Button::new(
        button,
        Text::new(text).horizontal_alignment(HorizontalAlignment::Center),
    )
    //Todo replace content of ToEpisode with some key
    .on_press(crate::Message::ToEpisodes(id))
    .padding(12)
    .width(Length::Fill)
}

impl List {
    fn view(&mut self, search_term: &str) -> Scrollable<crate::Message> {
        let mut scrollable = Scrollable::new(&mut self.scroll_state)
            .padding(10)
            .height(iced::Length::Fill);
        for (button, info) in self
            .feedres_buttons
            .iter_mut()
            .zip(self.feedres_info.iter())
        {
            scrollable = scrollable.push(feedres_button(button, info.clone()));
        }
        let valid_names = self
            .podcast_names
            .iter()
            .filter(|n| n.contains(search_term));
        for ((id, button), name) in izip!(self.podcast_buttons.iter_mut(), valid_names) {
            scrollable = scrollable.push(podcast_button(button, name.to_owned(), *id));
        }
        scrollable
    }
    pub fn down(&mut self) {
        self.scrolled_down += 10;
        self.scrolled_down = self.scrolled_down.min(self.podcast_buttons.len());
    }
    pub fn up(&mut self) {
        self.scrolled_down -= 10;
        self.scrolled_down = self.scrolled_down.max(0);
    }
    pub fn update_feedres(&mut self, results: Vec<feed::SearchResult>) {
        //TODO add feedres_buttons
        self.feedres_info = results;
        let needed_buttons = self
            .feedres_info
            .len()
            .saturating_sub(self.feedres_buttons.len());
        for _ in 0..needed_buttons {
            self.feedres_buttons.push(button::State::new());
        }
    }
    pub fn remove_feedres(&mut self) {
        self.feedres_info.clear();
    }
    pub fn add(&mut self, title: String, id: PodcastKey) {
        self.podcast_names.push(title);
        self.podcast_buttons.push((id, button::State::new()));
    }
}

pub struct Podcasts {
    /// the podcasts title
    pub list: List,
    pub search: Search,
    podcasts: database::PodcastDb,
    // possible opt to do, cache the view
}

impl Podcasts {
    pub fn from_db(db: database::PodcastDb) -> Self {
        let mut page = Podcasts {
            list: List::default(),
            search: Search::default(),
            podcasts: db,
        };
        for database::Podcast { title, .. } in page.podcasts.get_podcasts().unwrap() {
            let id = PodcastKey::from(title.as_str());
            page.list.podcast_names.push(title);
            page.list.podcast_buttons.push((id, button::State::new()));
        }
        page
    }
    pub fn down(&mut self) {
        self.list.down()
    }
    pub fn up(&mut self) {
        self.list.up()
    }
    pub fn view(&mut self) -> Element<crate::Message> {
        let scrollable = self.list.view(&self.search.input_value);
        let searchbar = self.search.view();

        let column = Column::new().push(searchbar).push(scrollable);
        column.into()
    }
}
