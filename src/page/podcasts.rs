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
pub struct PodcastSearch {
    input: text_input::State,
    input_value: String, 
}

impl PodcastSearch {
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
pub struct Podcasts {
    /// the podcasts title
    podcast_buttons: Vec<button::State>,
    podcast_names: Vec<String>,
    scroll_state: scrollable::State,
    podcast_search: PodcastSearch,
    // possible opt to do, cache the view
}

impl Podcasts {
    pub fn new() -> Self {
        let titles = ["99percentinvisible", "other_podcast"];
        let mut list = Podcasts::default();
        for title in titles.iter() {
            list.podcast_names.push(title.to_owned().to_string());
            list.podcast_buttons.push(button::State::new());
        }
        list
    }
    pub fn update(&mut self, message: Message) -> Command<crate::Message> {
        match message {
            Message::AddPodcast => {
                self.podcast_search.reset();
                Command::none()}
            Message::SearchInputChanged(_) => {
                self.podcast_search.update(message)
            }
        }
    }
    pub fn view(&mut self) -> Element<crate::Message> {
        let mut scrollable = Scrollable::new(&mut self.scroll_state)
            .padding(10)
            .height(iced::Length::Fill);
        for (button, name) in self.podcast_buttons.iter_mut()
            .zip(self.podcast_names.iter()) {

            scrollable = scrollable.push(
                Button::new(button, 
                    Text::new(name.to_owned()).horizontal_alignment(HorizontalAlignment::Center)
                )
                //Todo replace content of ToEpisode with some key
                .on_press(crate::Message::ToEpisodes(0)) //FIXME replace 0 with podcasts sled id
                .padding(12)
                .width(Length::Fill)
            )
        }
        let column = Column::new()
            .push(self.podcast_search.view())
            .push(scrollable);
        column.into()
    }
}
