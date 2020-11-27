use iced::Length;
use iced::{button, Button, Command, Element, Text, HorizontalAlignment};
use iced::widget::scrollable::{self, Scrollable};

#[derive(Debug, Clone)]
pub enum Message {
    Play(String),
}

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
    pub fn populate(&mut self, podcast: u64) {
        //TODO fetch podcast rss from db
        let example = include_bytes!("../99percentinvisible");
        let channel = rss::Channel::read_from(&example[..]).unwrap();

        self.episode_names.clear();
        self.episode_buttons.clear();
        for title in channel.items()
            .iter().filter_map(|x| x.title()) {

            self.episode_names.push(title.to_owned()); 
            self.episode_buttons.push(button::State::new());
        }
    }
    pub fn update(&mut self, message: Message) -> Command<crate::Message> {
        match message {
            Message::Play(episode_name) => {
                Command::none()
            }
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
                .on_press(crate::Message::Episodes(Message::Play(name.to_owned())))
                .padding(12)
                .width(Length::Fill)
            )
        }
        scrollable.into()
    }
}


