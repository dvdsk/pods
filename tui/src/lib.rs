use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::error::Error;
use std::io;
use std::sync::mpsc;
use presenter::{AppUpdate, UserIntent};

use tui::backend::{Backend, CrosstermBackend};
use tui::{Frame, Terminal};

struct App {}

impl App {
    fn new() -> App {
        App {}
    }
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    rx: &mut mpsc::Receiver<AppUpdate>,
    tx: &mut mpsc::Sender<UserIntent>,
) -> std::io::Result<()> {
    use crossterm::event::{self, Event, KeyCode};
    loop {
        terminal.draw(|f| ui(f))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => tx.send(UserIntent::Exit).unwrap(),
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
    rx: mpsc::Receiver<AppUpdate>,
    tx: mpsc::Sender<UserIntent>,
}

pub fn new(rx: mpsc::Receiver<AppUpdate>, tx: mpsc::Sender<UserIntent>) -> Tui {
    Tui { rx, tx }
}

impl presenter::Ui for Tui {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let app = App::new();
        let res = run_app(&mut terminal, &mut self.rx, &mut self.tx);

        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("{:?}", err)
        }

        Ok(())
    }
}
