use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use color_eyre::eyre;
use iced::{executor, widget, Application, Subscription};
use presenter::{ActionDecoder, GuiUpdate, Presenter};

struct State {
    rx: Arc<Mutex<Presenter>>,
    tx: ActionDecoder,
    should_exit: bool,
}

#[derive(Debug)]
pub enum Message {
    Gui(GuiUpdate),
}

#[derive(Debug)]
enum Event {}

type Command = iced::Command<Message>;

impl Application for State {
    type Message = Message;
    type Executor = executor::Default;
    type Theme = iced::Theme;
    type Flags = (Presenter, ActionDecoder);

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
            Message::Gui(GuiUpdate::Exit) => self.should_exit = true,
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

fn update_sub(rx: Arc<Mutex<Presenter>>) -> iced::Subscription<Message> {
    struct GetUpdates;
    let id = std::any::TypeId::of::<GetUpdates>();
    iced::subscription::unfold(id, rx, move |rx| async {
        let msg = {
            let mut presenter = rx.try_lock().expect("locking should always succeed");
            let update = presenter.update().await;
            let msg = Message::Gui(update);
            msg
        };
        (Some(msg), rx)
    })
}

pub struct IcedGui {
    rx: Option<Presenter>,
    tx: Option<ActionDecoder>,
}

pub fn new(interface: presenter::Interface) -> Box<dyn presenter::Ui> {
    let (tx, rx) = interface;
    Box::new(IcedGui {
        rx: Some(rx),
        tx: Some(tx),
    })
}

#[async_trait]
impl presenter::Ui for IcedGui {
    async fn run(&mut self) -> Result<(), eyre::Report> {
        let settings =
            iced::Settings::with_flags((self.rx.take().unwrap(), self.tx.take().unwrap()));
        tokio::task::block_in_place(|| State::run(settings))?;
        Ok(())
    }
}
