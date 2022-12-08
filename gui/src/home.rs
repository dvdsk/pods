use crate::menu;
use crate::Page;
use iced::widget;

use crate::Message;
pub fn view(column: widget::Column<'static, Message>) -> widget::Column<'static, Message> {
    column.push(menu::button("podcasts", Message::ToPage(Page::Podcasts)))
}
