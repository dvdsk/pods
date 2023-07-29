use crate::{menu, Message};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{self, Column, Scrollable, Text, TextInput};
use iced::{Element, Length};
use traits::SearchResult;

pub type Podcasts = Vec<traits::Podcast>;
pub(crate) type ResultIdx = usize;

#[derive(Debug)]
pub struct Details {
    item: ResultIdx,
    new_results: Vec<SearchResult>,
}

#[derive(Debug, Default)]
pub struct Search {
    results: Vec<SearchResult>,
    details: Option<Details>,
    query: String,
}

fn view_details(idx: ResultIdx, res: &SearchResult) -> Element<'static, Message> {
    let mut list = widget::Column::new();
    let on_click = Message::SearchDetailsClose;
    list = list.push(menu::button(res.title.clone(), on_click));
    let on_click = Message::AddPodcast(idx);
    list = list.push(menu::button("ADD", on_click));
    list.into()
}

impl Search {
    pub fn view(
        &self,
        mut column: widget::Column<'static, Message>,
    ) -> widget::Column<'static, Message> {
        let on_change = Message::SearchUpdate;
        let input = TextInput::new("search", &self.query, on_change);
        column = column.push(input);

        let mut list = Column::new();
        for (idx, res) in self.results.iter().enumerate() {
            if let Some(details) = &self.details {
                if details.item == idx {
                    list = list.push(view_details(idx, res));
                    continue;
                }
            }

            let on_click = Message::SearchDetails(idx);
            list = list.push(menu::button(res.title.clone(), on_click));
        }
        let list = Scrollable::new(list);
        column = column.push(list);

        column
    }

    pub(crate) fn update_results(&mut self, res: Vec<SearchResult>) {
        if let Some(details) = &mut self.details {
            details.new_results = res;
        } else {
            self.results = res;
        }
    }

    pub(crate) fn open_details(&mut self, item: ResultIdx) {
        self.details = Some(Details {
            item,
            new_results: Vec::new(),
        });
    }

    pub(crate) fn close_details(&mut self) {
        if let Some(Details { new_results, .. }) = self.details.take() {
            if !new_results.is_empty() {
                self.results = new_results;
            }
        }
    }

    pub(crate) fn add_podcast(&self, idx: usize, tx: &mut presenter::ActionDecoder) {
        tx.add_podcast(self.results[idx].clone())
    }

    pub(crate) fn update_query(&mut self, query: String, tx: &mut presenter::ActionDecoder) {
        self.query = query.clone();
        tx.search_enter(query);
    }
}

fn text<'a>(text: String) -> Text<'a> {
    widget::Text::new(text)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center)
}
