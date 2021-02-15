use iced::{button, Button, Element, Text, HorizontalAlignment, Row, Column, Container};
use iced::widget::container::Style;
use crate::database::{Progress, Date};
use crate::download::FileType;
use crate::Message;
use super::style;

pub struct Collapsed {
    title: String,
    age: String,
    duration: String,
    date: Date,
    progress: Progress,
    file: Option<FileType>,
    enqueue: button::State,
}

fn enqueue_button(state: &mut button::State) -> Button<Message> {
    let icon = Text::new("+".to_string())
        .horizontal_alignment(HorizontalAlignment::Center)
        .size(20);
    Button::new(state, icon)
        .on_press(Message::None) //TODO
}

impl Collapsed {
    fn age(&self) -> Text {
        Text::new(&self.age)
            .horizontal_alignment(HorizontalAlignment::Left)
            .size(10)
    }
    fn duration(&self) -> Text {
        Text::new(&self.duration)
            .horizontal_alignment(HorizontalAlignment::Right)
            .size(10)
    }
    fn title(&self) -> Text {
        Text::new(&self.title)
            .horizontal_alignment(HorizontalAlignment::Left)
            .size(10)
    }

    pub fn view(&mut self, theme: style::Theme) -> Element<crate::Message> {
        let meta = Column::new()
            .push(self.age())
            .push(self.duration());

        let column = Row::new()
            .push(self.title())
            .push(meta);

        let row = Column::new()
            .push(column)
            .push(enqueue_button(&mut self.enqueue))
            .into();
        let element = Container::new(row)
            .style(theme)
            .into();
        element
    }
}

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
