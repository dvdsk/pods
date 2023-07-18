use crate::Message;
use iced::widget;
use presenter::ActionDecoder;

#[derive(Default)]
pub struct Podcasts {}

pub fn load(tx: &mut ActionDecoder) {
    tx.view_podcasts();
}

pub fn view(
    column: widget::Column<'static, Message>,
    podcasts: &Podcasts,
) -> widget::Column<'static, Message> {
    column
}

