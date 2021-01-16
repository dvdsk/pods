mod page;
mod database;
mod feed;
mod play;
use page::Page;
use play::Player;

use iced::{button, executor, Application, Command, Element, Column, Settings, Subscription};

#[derive(Clone, Debug)]
pub enum Message {
    ToEpisodes(u64),
    PlayProgress(f32),
    Play(database::episodes::Key),
    // PlayCallback(play::WebToDecoderStream),
    Download(database::episodes::Key),
    Back,
    Pauze,
    Resume,
    Podcasts(page::podcasts::Message),
    AddPodcast(String), //url
    StreamProgress(play::subscribe::Progress),
    Skip(f32),
    // Episodes(page::episodes::Message),
}

use std::sync::Arc;
pub struct App {
    current: Page,
    podcasts: page::Podcasts,
    episodes: page::Episodes,
    player: Player,
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
            player: Player::from_db(&episode_db), 
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
                self.episodes.populate(podcast_id, list);
                self.current = Page::Episodes;
                Command::none()
            }
            Message::StreamProgress(p) => {
                use play::subscribe::Progress;
                match p {
                    Progress::ToShortError => log::warn!("stream was to short to play audio"),
                    Progress::StreamError(e) => log::error!("errored stream {}",e),
                    Progress::Started(rx) => self.player.rx = Some(rx),
                    Progress::Finished => self.player.current = play::Track::None,
                    Progress::Advanced(p) => {
                        self.player.current.set_streampos(p);
                        if self.player.sink.empty() && p > 10f32 {
                            let rx = self.player.rx.take().unwrap();
                            let rx = Arc::try_unwrap(rx).unwrap();
                            let rx = rx.into_inner().unwrap();
                            let rrx = play::ReadableReciever::new(rx);
                            let source = rodio::Decoder::new_mp3(rrx).unwrap();
                            self.player.sink.append_seekable(source);
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
                self.player.play_stream(key);
                Command::none()
            }
            Message::Skip(f) => {
                self.player.skip(f);
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
        if let play::Track::Stream(_,_,url) = &self.player.current {
            log::info!("playing");
            play::subscribe::play(url.to_owned()).map(Message::StreamProgress)
        } else {
            Subscription::none()
        }
    }
    fn view(&mut self) -> Element<Self::Message> {
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
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
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
