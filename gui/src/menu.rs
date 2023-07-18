use crate::icon;
use crate::Message;
use crate::Page;
use iced::{
    alignment::{Horizontal, Vertical},
    widget, Length,
};

pub fn button(text: &'static str, event: Message) -> widget::Button<'static, Message> {
    let text = widget::Text::new(text)
        .width(Length::Fill)
        .height(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);
    widget::button(text).on_press(event).width(Length::Fill)
}

pub fn view_bar(in_menu: bool) -> widget::Column<'static, Message> {
    let button = match in_menu {
        true => widget::button(icon::close_menu())
            .on_press(Message::CloseMenu)
            .width(Length::Fill),
        false => widget::button(icon::open_menu())
            .on_press(Message::OpenMenu)
            .width(Length::Fill),
    };
    let row = widget::Row::new().push(button);
    widget::Column::new().push(row)
}

pub fn view(column: widget::Column<'static, Message>) -> widget::Column<'static, Message> {
    column
        .push(button("home", Message::ToPage(Page::Home)))
        .push(button("search", Message::ToPage(Page::Search)))
        .push(button("settings", Message::ToPage(Page::Settings)))
        .push(button("downloads", Message::ToPage(Page::Downloads)))
        .push(button("playlists", Message::ToPage(Page::Playlists)))
}