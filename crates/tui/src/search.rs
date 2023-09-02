use crossterm::event::{Event, KeyCode, KeyEvent};
use presenter::ActionDecoder;
use tui::layout::Alignment;
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui_input::Input;

use crate::State;

#[derive(Default)]
pub struct Search {
    input: Input,
    searching: bool,
}

impl Search {
    pub fn render(&self) -> Paragraph {
        let text = if self.input.value().is_empty() {
            "Press / to search"
        } else {
            &self.input.value()
        };

        Paragraph::new(text)
            .block(Block::default().title("Search").borders(Borders::ALL))
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
    }

    pub(crate) async fn update(
        &mut self,
        state: super::State,
        key: KeyEvent,
        key_event: Event,
        tx: &mut ActionDecoder,
    ) -> Option<super::State> {
        use tui_input::backend::crossterm as input_backend;

        match (key.code, state) {
            (KeyCode::Char('/'), State::Normal) => Some(State::EditingSearch),
            (_, State::Normal) => None,
            (KeyCode::Enter, State::EditingSearch) => {
                tx.search_enter(self.input.value().to_owned());
                self.searching = true;
                Some(State::Normal)
            }
            (KeyCode::Esc, State::EditingSearch) => {
                self.input.reset();
                Some(State::Normal)
            }
            (_, State::EditingSearch) => {
                if let Some(req) = input_backend::to_input_request(key_event) {
                    self.input.handle(req);
                }
                Some(State::EditingSearch)
            }
        }
    }
}
