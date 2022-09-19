use async_trait::async_trait;
use color_eyre::eyre;
use color_eyre::eyre::WrapErr;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use presenter::{ActionDecoder, AppUpdate, Presenter, UserAction, UserIntent};
use std::io;

use tui::backend::{Backend, CrosstermBackend};
use tui::{Frame, Terminal};

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    rx: &mut Presenter,
    tx: &mut ActionDecoder,
) -> eyre::Result<()> {
    use crossterm::event::{self, Event, KeyCode};
    loop {
        terminal.draw(|f| ui(f))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char(q) => tx.decode(UserAction::KeyPress(q)).await,
                _ => (),
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>) {
    use tui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Style},
        widgets::{Block, Borders, Paragraph},
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(3)].as_ref())
        .split(f.size());

    let input = Paragraph::new("hello world")
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input"));

    f.render_widget(input, chunks[0]);
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
