use std::error::Error;

use iced::{executor, widget, Application};
use traits::Ui;

struct State {}

#[derive(Debug)]
pub enum Message {}

#[derive(Debug)]
enum Event {}

type Command = iced::Command<Message>;

impl Application for State {
    type Message = Message;
    type Executor = executor::Default;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (State, Command) {
        (State {}, Command::none())
    }

    fn title(&self) -> String {
        String::from("Panda Podcast")
    }

    fn update(&mut self, _message: Self::Message) -> Command {
        Command::none()
    }

    fn view(&self) -> iced::Element<Message> {
        widget::text("Hello world").into()
    }
}

pub struct IcedGui {}

pub fn new() -> IcedGui {
    IcedGui {}
}

impl Ui for IcedGui {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(State::run(iced::Settings::default())?)
    }
}
