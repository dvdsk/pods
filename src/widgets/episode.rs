use std::collections::HashMap;

use iced::{button, Button, Align, Length, Space, Element, Text, HorizontalAlignment, Row, Column, Container};

use crate::database::{Episode, Progress, Date};
use crate::download::FileType;
use crate::Message;
use super::style;

#[derive(Debug)]
pub struct Collapsed {
    pub title: String,
    age: String,
    duration: String,
    date: Date,
    progress: Progress,
    pub file: Option<FileType>,
    expand: button::State,
    enqueue: button::State,
}

fn enqueue_button(state: &mut button::State) -> Button<Message> {
    let icon = Text::new("+".to_string())
        .horizontal_alignment(HorizontalAlignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .size(40);
    Button::new(state, icon)
        .on_press(Message::None)
}

impl Collapsed {
    fn age(&self) -> Text {
        Text::new(format!("{} ", &self.age))
            .horizontal_alignment(HorizontalAlignment::Left)
            .size(25)
    }
    fn duration(&self) -> Text {
        Text::new(&self.duration)
            .horizontal_alignment(HorizontalAlignment::Right)
            .size(25)
    }
    fn title(&self) -> Text {
        Text::new(&self.title)
            .horizontal_alignment(HorizontalAlignment::Left)
            .size(40)
    }

    pub fn view(&mut self, theme: style::Theme) -> Element<crate::Message> {
        let meta = Row::new()
            .push(self.age())
            .push(Space::with_width(Length::Fill))
            .push(self.duration());

        let column = Column::new()
            .push(self.title())
            .push(meta);
        let column = Button::new(&mut self.expand, column)
            // .style(style::Clear)
            .on_press(Message::None);
        let column = Container::new(column)
            .width(Length::FillPortion(2));
            // .style(theme);

        let row = Row::new()
            .push(column)
            .push(enqueue_button(&mut self.enqueue))
            .spacing(10)
            .align_items(Align::Start);

        row.into()
    }

    pub fn from(episode: Episode, episodes_on_disk: &HashMap<u64, FileType>) -> Self {
        use crate::download::hash;

        let file = episodes_on_disk
            .get(&hash(&episode.title))
            .copied();

        Self {
            title: episode.title,
            age: String::from("2 weeks"),
            duration: String::from("35m"),
            date: episode.date,
            progress: episode.progress,
            file, // none if no file on disk
            expand: button::State::new(),
            enqueue: button::State::new(),
        }
    }
}

#[derive(Debug)]
pub struct Expanded {
    collapsed: Collapsed,
    description: String,
    stream: button::State,
    add_to_pl: button::State,
    remove: button::State,
}

fn vbar() -> Text {
    Text::new(" | ")
        .size(10)
}

impl Expanded {

    pub fn from_collapsed(collapsed: Collapsed, description: String) -> Self {
        Expanded {
            collapsed,
            description,
            stream: button::State::new(),
            add_to_pl: button::State::new(),
            remove: button::State::new(),
        }
    }
    pub fn view(&mut self, theme: style::Theme) -> Element<crate::Message> {
        let Self {collapsed, description, stream, add_to_pl, remove} = self;

        let buttons = Column::new()
            .push(small_button(stream, "Stream", Message::None))
            .push(vbar())
            .push(small_button(add_to_pl, "Add to playlist", Message::None))
            .push(vbar())
            .push(small_button(remove, "Remove", Message::None));

        let row = Row::new()
            .push(collapsed.view(theme))
            .push(description_text(description))
            .push(buttons)
            .into();
        row
    }
}

fn description_text(description: &String) -> Text {
    Text::new(description)
        .horizontal_alignment(HorizontalAlignment::Left)
        .size(10)
}

fn small_button<'a>(state: &'a mut button::State, text: &str, message: Message) -> Button<'a, Message> {
    let text = Text::new(text)
        .horizontal_alignment(HorizontalAlignment::Center)
        .size(20);
    Button::new(state, text)
        .on_press(message)
}
