use std::error::Error;
use std::sync::{mpsc, Mutex, Arc};

use iced::{executor, widget, Application, Subscription};
use presenter::{AppUpdate, UserIntent};

struct State {
    rx: Arc<Mutex<mpsc::Receiver<AppUpdate>>>,
    tx: mpsc::Sender<UserIntent>,
    should_exit: bool,
}

#[derive(Debug)]
pub enum Message {
    App(AppUpdate),
}

#[derive(Debug)]
enum Event {}

type Command = iced::Command<Message>;

impl Application for State {
    type Message = Message;
    type Executor = executor::Default;
    type Theme = iced::Theme;
    type Flags = (mpsc::Receiver<AppUpdate>, mpsc::Sender<UserIntent>);

    fn new((rx, tx): Self::Flags) -> (State, Command) {
        (
            State {
                rx: Arc::new(Mutex::new(rx)),
                tx,
                should_exit: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Panda Podcast")
    }

    fn should_exit(&self) -> bool {
        dbg!(self.should_exit)
    }

    fn update(&mut self, message: Self::Message) -> Command {
        match dbg!(message) {
            Message::App(AppUpdate::Exit) => self.should_exit = true,
        }
        Command::none()
    }

    fn view(&self) -> iced::Element<Message> {
        widget::text("Hello world").into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        update_sub(self.rx.clone())
    }
}

fn update_sub(rx: Arc<Mutex<mpsc::Receiver<AppUpdate>>>) -> iced::Subscription<Message> {
    struct GetUpdates;
    let id = std::any::TypeId::of::<GetUpdates>();
    iced::subscription::unfold(id, rx, move |rx| async {
        dbg!();
        let update = rx
            .try_lock()
            .expect("locking should always succeed")
            .recv()
            .unwrap(); //.await;
        let msg = Message::App(update);
        (Some(msg), rx)
    })
}

pub struct IcedGui {
    rx: Option<mpsc::Receiver<AppUpdate>>,
    tx: Option<mpsc::Sender<UserIntent>>,
}

pub fn new(rx: mpsc::Receiver<AppUpdate>, tx: mpsc::Sender<UserIntent>) -> IcedGui {
    IcedGui {
        rx: Some(rx),
        tx: Some(tx),
    }
}

impl presenter::Ui for IcedGui {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let settings =
            iced::Settings::with_flags((self.rx.take().unwrap(), self.tx.take().unwrap()));
        Ok(State::run(settings)?)
    }
}
