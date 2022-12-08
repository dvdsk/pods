use crate::Message;
use crate::Page;
use iced::{
    alignment::{Horizontal, Vertical},
    widget, Length,
};

pub fn button(text: &'static str, event: Message) -> widget::Button<'static, Message> {
    let text = widget::Text::new(text)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);
    widget::button(text).on_press(event).width(Length::Fill)
}

pub fn icon(text: &'static str, event: Message) -> widget::Button<'static, Message> {
    let text = widget::Text::new(text)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);
    widget::button(text).on_press(event).width(Length::Fill)
}

pub fn view_bar(in_menu: bool) -> widget::Column<'static, Message> {
    let button = match in_menu {
        true => icon("X", Message::CloseMenu),
        false => icon("M", Message::OpenMenu),
    };
    let row = widget::Row::new().push(button);
    widget::Column::new().push(row)
}

pub fn view(column: widget::Column<'static, Message>) -> widget::Column<'static, Message> {
    column
        .push(button("search", Message::ToPage(Page::Search)))
        .push(button("settings", Message::ToPage(Page::Settings)))
        .push(button("downloads", Message::ToPage(Page::Downloads)))
        .push(button("playlists", Message::ToPage(Page::Playlists)))
}