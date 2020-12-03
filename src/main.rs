mod page;
mod database;
mod feed;
mod play;
use page::Page;
use play::PlayBack;

use iced::{button, executor, Application, Command, Element, Column, Settings, Subscription};

#[derive(Clone, Debug)]
pub enum Message {
    ToEpisodes(u64),
    PlayProgress(f32),
    Play(database::episodes::Key),
    // PlayCallback(play::WebToDecoderStream),
    Download((String,String)),
    Back,
    Pauze,
    Resume,
    Podcasts(page::podcasts::Message),
    AddPodcast(String), //url
    StreamProgress(play::subscribe::Progress),
    // Episodes(page::episodes::Message),
}

use std::sync::{mpsc, Arc, Mutex};
pub struct App {
    current: Page,
    podcasts: page::Podcasts,
    episodes: page::Episodes,
    player: PlayBack,
    back_button: button::State, //Should only be needed on desktop platforms
    pod_db: database::Podcasts,
    episode_db: database::Episodes,
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (App, Command<Self::Message>) {
        let db = database::open().unwrap();
        let pod_db = database::Podcasts::open(&db).unwrap();
        let episode_db = database::Episodes::open(&db).unwrap();
        (App {
            podcasts: page::Podcasts::from_db(pod_db.clone()),
            episodes: page::Episodes::from_db(&episode_db), 
            current: Page::Podcasts,
            player: PlayBack::from_db(&episode_db), 
            back_button: button::State::new(),
            pod_db,
            episode_db,
        }, Command::none())
    }
    fn title(&self) -> String {
        String::from("Podcasts")
    }
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Back => {
                self.current.back();
                Command::none()
            }
            Message::ToEpisodes(podcast_id) => {
                let list = self.pod_db.get_episodelist(podcast_id).unwrap();
                self.episodes.populate(list);
                self.current = Page::Episodes;
                Command::none()
            }
            Message::StreamProgress(p) => {
                use play::subscribe::Progress;
                match p {
                    Progress::Errored => {dbg!("errored stream"); ()}
                    Progress::Started(rx) => self.player.rx = Some(rx),
                    Progress::Finished => self.player.status = None,
                    Progress::Advanced(p) => {
                        self.player.status.as_mut().unwrap().pos = p;
                        if self.player.sink.empty() && p > 10f32 {
                            let rx = self.player.rx.take().unwrap();
                            let rx = Arc::try_unwrap(rx).unwrap();
                            let rx = rx.into_inner().unwrap();
                            let rrx = play::ReadableReciever::new(rx);
                            let source = rodio::Decoder::new_mp3(rrx).unwrap();
                            self.player.sink.append(source);
                            dbg!("**starting playback**");
                        }
                    }
                }
                Command::none()
            }
            Message::PlayProgress(p) => {
                // self.player.as_mut().unwrap().pos = p;
                Command::none()
            }
            Message::AddPodcast(url) => {
                let pod_db = self.pod_db.clone();
                let ep_db = self.episode_db.clone();
                Command::perform(
                    feed::add_podcast(pod_db, ep_db, url), 
                    |x| Message::Podcasts(page::podcasts::Message::AddedPodcast(x.0,x.1)))
            }
            Message::Play(key) => {
                dbg!();
                self.player.status = Some(play::Status::default());
                Command::none()
            }
            // Message::PlayCallback(stream) => {
            //     Command::perform(
            //         play::continue_streaming(stream),
            //         Message::
            // }
            Message::Download(key) => Command::none(),
            Message::Pauze => Command::none(),
            Message::Resume => Command::none(),
            Message::Podcasts(m) => self.podcasts.update(m),
            // Message::Episodes(m) => self.episodes.update(m),
        }
    }
    fn subscription(&self) -> Subscription<Self::Message> {
        let url = String::from("https://file-examples-com.github.io/uploads/2017/11/file_example_MP3_5MG.mp3");
        if let Some(s) = &self.player.status {
            play::subscribe::play(url).map(Message::StreamProgress)
        } else {
            Subscription::none()
        }
    }

    fn view(&mut self) -> Element<Self::Message> {
        dbg!("view");
        dbg!(&self.current);
        let content = match self.current {
            Page::Podcasts => self.podcasts.view(), // TODO load from a cache
            Page::Episodes => self.episodes.view(),
        };
        let column = Column::new();
        let column = column.push(content);
        let column = column.push(self.player.view());
        #[cfg(feature = "desktop")]
        let column = if self.current != Page::Podcasts {
            column.push(page::draw_back_button(&mut self.back_button))
        } else {column};
        
        iced::Container::new(column).into()
    }
}

pub fn main() -> iced::Result {
    let settings = build_settings();
    App::run(settings)
}

fn build_settings() -> Settings<()> {
    Settings {
        window: iced::window::Settings::default(),
        flags: (),
        default_font: None,
        default_text_size: 20,
        antialiasing: false,
    }
}
