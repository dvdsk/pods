use crate::{menu, Message};
use iced::widget::{self, Column, Scrollable, TextInput};
use traits::SearchResult;

use super::super::State;

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
}

pub(crate) fn update_query(state: &mut State, query: String) {
    state.search.query = query.clone();
    state.tx.search_enter(query);
}
