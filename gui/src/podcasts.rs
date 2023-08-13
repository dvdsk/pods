pub mod add;

use crate::{menu, Loading, Message, Page};
use iced::widget::{self, Column, Scrollable};
use traits::DataUpdateVariant;

pub type Podcasts = Vec<traits::Podcast>;

pub(super) fn load(state: &mut super::State) {
    state.tx.view_podcasts();
    state.loading = Some(Loading::new(Page::Podcasts, [DataUpdateVariant::Podcast]));
}

pub fn view(
    mut column: widget::Column<'static, Message>,
    podcasts: &Podcasts,
) -> widget::Column<'static, Message> {
    let mut list = Column::new();
    for podcast in podcasts {
        let on_click = Message::ToPage(Page::Podcast {
            id: 0,
            details: None,
        });
        list = list.push(menu::button(podcast.name.clone(), on_click));
    }

    let list = Scrollable::new(list);
    column = column.push(list);

    let add_podcast = menu::button("+", Message::ToPage(Page::AddPodcast));
    column = column.push(add_podcast);

    column
}
