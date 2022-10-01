use async_trait::async_trait;
use futures::{FutureExt, StreamExt, TryStreamExt};

use color_eyre::eyre;
use color_eyre::eyre::WrapErr;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use presenter::{ActionDecoder, GuiUpdate, Presenter, UserAction};
use std::io;

use tui::backend::{Backend, CrosstermBackend};
use tui::{Frame, Terminal};

use crate::search::Search;

mod search;

#[derive(Default)]
struct App {
    state: State,
    search: Search,
}

#[derive(Default, Clone, Copy)]
enum State {
    #[default]
    Normal,
    EditingSearch,
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    rx: &mut Presenter,
    tx: &mut ActionDecoder,
) -> eyre::Result<()> {
    use crossterm::event::EventStream;
    let mut events = EventStream::new().fuse();
    let mut app = App::default();

    terminal.draw(|f| ui(f, &app))?;
    loop {
        let exit = futures::select! {
            update = rx.update().fuse() => handle_update(update),
            event = events.try_next() => {
                let event = event.expect("Error in TUI input").expect("TUI stopped sending input");
                handle_tui_event(event, tx, &mut app).await?;
                false
            }
        };

        if exit {
            return Ok(());
        }

        terminal.draw(|f| ui(f, &app))?;
    }
}

fn handle_update(update: GuiUpdate) -> bool {
    match update {
        GuiUpdate::Exit => true,
    }
}

async fn handle_tui_event(event: Event, tx: &mut ActionDecoder, app: &mut App) -> eyre::Result<()> {
    let key = match event {
        Event::Key(key) => key,
        _ => return Ok(()),
    };

    let App { state, search } = app; 
    // TODO: when async closures stablize make this into a chain using or_else(|| ....
    // update).or_else(|| ...) etc etc <dvdsk noreply@davidsk.dev> 
    if let Some(new_state) = search.update(*state, key, event, tx).await {
        *state = new_state;
        return Ok(())
    }
    if let Some(new_state) = App::update(*state, key, tx).await {
        *state = new_state;
        return Ok(())
    }

    tracing::warn!("unhandled key: {key:?}");

    Ok(())
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    use tui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Style},
        widgets::{Block, Borders, Paragraph},
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(3), Constraint::Length(3)].as_ref())
        .split(f.size());

    let input = Paragraph::new("hello world")
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input"));

    f.render_widget(app.search.render(), chunks[0]);
    f.render_widget(input, chunks[1]);
}

pub struct Tui {
    rx: Presenter,
    tx: ActionDecoder,
}

pub fn new(interface: presenter::InternalPorts) -> Box<dyn presenter::Ui> {
    let presenter::InternalPorts(tx, rx) = interface;
    Box::new(Tui { rx, tx })
}

#[async_trait]
impl presenter::Ui for Tui {
    async fn run(&mut self) -> Result<(), eyre::Report> {
        enable_raw_mode().wrap_err("Could not enable raw terminal mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .wrap_err("Could not enable mouse capture")?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).wrap_err("Could not wrap terminal")?;

        let res = run_app(&mut terminal, &mut self.rx, &mut self.tx).await;

        // restore terminal
        disable_raw_mode().wrap_err("Could not disable raw terminal")?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .wrap_err("Could not disable mouse capture and leave alternate screen")?;
        terminal
            .show_cursor()
            .wrap_err("Could not re-enable cursor")?;

        if let Err(err) = res {
            println!("{:?}", err)
        }

        Ok(())
    }
}
impl App {
    async fn update(
        state: State,
        key: crossterm::event::KeyEvent,
        tx: &mut ActionDecoder,
    ) -> Option<State> {
        match key.code {
            KeyCode::Char(q) => {
                tx.decode(UserAction::KeyPress(q)).await;
                return Some(state)
            }
            _ => (),
        }
        None
    }
}
