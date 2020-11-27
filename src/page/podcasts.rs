use iced::Length;
use iced::{button, Button, Column, Command, Element, Text, HorizontalAlignment};
use iced::{scrollable, Scrollable};
use iced::{TextInput, text_input};

#[derive(Debug, Clone)]
pub enum Message {
    AddPodcast,
    SearchInputChanged(String),
}

impl Into<crate::Message> for Message {
    fn into(self) -> crate::Message {
        crate::Message::Podcasts(self)
    }
}

#[derive(Default)]
pub struct Search {
    input: text_input::State,
    input_value: String, 
}

impl Search {
    pub fn update(&mut self, message: Message) -> Command<crate::Message> {
        match message {
            Message::AddPodcast => Command::none(),
            Message::SearchInputChanged(s) => {
                self.input_value = s;
                Command::none() 
            }
        }
    }
    pub fn view(&mut self) -> TextInput<crate::Message> {
        TextInput::new(
            &mut self.input, 
            "Add podcast url", 
            &self.input_value, 
            |s| crate::Message::Podcasts(Message::SearchInputChanged(s)),
        ) 
        .width(Length::Fill)
        .on_submit(crate::Message::Podcasts(Message::AddPodcast))
    }
    pub fn reset(&mut self) {
        self.input_value.clear();
    }
}

#[derive(Default)]
struct List {
    podcast_buttons: Vec<button::State>,
    podcast_names: Vec<String>,
    scroll_state: scrollable::State,
}

impl List {
    fn view(&mut self, search_term: &str) -> Scrollable<crate::Message> {
        let mut scrollable = Scrollable::new(&mut self.scroll_state)
            .padding(10)
            .height(iced::Length::Fill);
        for (button, name) in self.podcast_buttons.iter_mut()
            .zip(self.podcast_names.iter().filter(|n| n.contains(search_term))) {

            scrollable = scrollable.push(
                Button::new(button, 
                    Text::new(name.to_owned()).horizontal_alignment(HorizontalAlignment::Center)
                )
                //Todo replace content of ToEpisode with some key
                .on_press(crate::Message::ToEpisodes(0))
                .padding(12)
                .width(Length::Fill)
            )
        }
        scrollable
    }
}

#[derive(Default)]
pub struct Podcasts {
    /// the podcasts title
    list: List,
    search: Search,
    // possible opt to do, cache the view
}

impl Podcasts {
    pub fn new() -> Self {
        let titles = ["99percentinvisible", "other_podcast"];
        let mut page = Podcasts::default();
        for title in titles.iter() {
            page.list.podcast_names.push(title.to_owned().to_string());
            page.list.podcast_buttons.push(button::State::new());
        }
        page
    }
    pub fn update(&mut self, message: Message) -> Command<crate::Message> {
        match message {
            Message::AddPodcast => {
                self.search.reset();
                Command::none()}
            Message::SearchInputChanged(_) => {
                self.search.update(message)
            }
        }
    }
    pub fn view(&mut self) -> Element<crate::Message> {
        let scrollable = self.list.view(&self.search.input_value);
        let search = self.search.view();

        let column = Column::new()
            .push(search)
            .push(scrollable);
        column.into()
    }
}
