mod page;
mod database;
mod feed;
mod play;
mod download;

use download::Downloader;
use page::{Page, Controls};
use play::Player;
use error_level::ErrorLevel;
use database::episodes::Key as EpisodeKey;
use database::podcasts::EpisodeList;
use page::episodes::FileType;

use std::collections::HashMap;
use iced::{button, executor, Application, Command, Element, Column, Settings, Subscription};

#[derive(Clone, Debug)]
pub enum Message {
    ToEpisodes(u64),
    ToEpisodesFinish(HashMap<u64, FileType>, EpisodeList, u64),
    PlayBackTick(std::time::Instant),
    Stream(EpisodeKey),
    Play(EpisodeKey, FileType),
    // PlayCallback(play::WebToDecoderStream),
    Download(EpisodeKey),
    Remove(EpisodeKey, FileType),
    Back,
    Up,
    Down,
    PlayPause,
    Podcasts(page::podcasts::Message),
    AddPodcast(String), //url
    StreamProgress(play::subscribe::Progress),
    DownloadProgress(download::Progress),
    DownloadFinished(HashMap<u64, FileType>),
    Skip(f32),
    // Episodes(page::episodes::Message),
}

pub struct App {
    current: Page,
    podcasts: page::Podcasts,
    episodes: page::Episodes,
    downloader: Downloader,
    player: Player,
    controls: Controls, //Should only be needed on desktop platforms
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
            downloader: Downloader::default(),
            controls: Controls::default(),
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
            Message::Up => {
                match &self.current {
                    Page::Podcasts => self.podcasts.up(),
                    Page::Episodes => self.episodes.up(),
                }
                Command::none()
            }
            Message::Down => {
                match &self.current {
                    Page::Podcasts => self.podcasts.down(),
                    Page::Episodes => self.episodes.down(),
                }
                Command::none()
            }
            Message::ToEpisodes(podcast_id) => {
                let list = self.pod_db.get_episodelist(podcast_id).unwrap();
                Command::perform(
                    page::episodes::scan_podcast_wrapper(list),
                    move |(set,list)| Message::ToEpisodesFinish(set,list, podcast_id),
                )
            }
            Message::ToEpisodesFinish(downloaded, list, podcast_id) => {
                self.episodes.populate(list, podcast_id, downloaded);
                self.current = Page::Episodes;
                Command::none()
            }
            Message::StreamProgress(p) => {
                use play::subscribe::Progress;
                match p {
                    Progress::ToShortError => log::warn!("stream was to short to play audio"),
                    Progress::StreamError(e) => log::error!("errored stream {}",e),
                    Progress::Started(rx) => self.player.rx = Some(rx),
                    Progress::Finished => (),
                    Progress::Advanced(p) => {
                        self.player.current.set_streampos(p);
                        if self.player.stream_ready(p) {
                            self.player.start_stream();
                        }
                    }
                }
                Command::none()
            }
            Message::DownloadProgress(p) => {
                use download::Progress::*;
                match p {
                    Error(e) => {
                        e.log_error();
                        Command::none()
                    }
                    Finished => {
                        log::info!("finished download");
                        let podcast = self.episodes.podcast.as_ref().unwrap().clone();
                        Command::perform(
                            page::episodes::scan_podcast_dir(podcast),
                            Message::DownloadFinished
                        )
                    }
                    _ => Command::none(),
                }
            }
            Message::DownloadFinished(set) => {
                self.episodes.update_downloaded(set);
                Command::none()
            }
            Message::PlayBackTick(_) => {
                // only used to trigger a redraw
                Command::none()
            }
            Message::AddPodcast(url) => {
                let pod_db = self.pod_db.clone();
                let ep_db = self.episode_db.clone();
                Command::perform(
                    feed::add_podcast(pod_db, ep_db, url), 
                    |x| Message::Podcasts(page::podcasts::Message::AddedPodcast(x.0,x.1)))
            }
            Message::Stream(key) => {
                self.player.add_stream(key);
                Command::none()
            }
            Message::Play(key, file_type) => {
                self.player.add_file(key, file_type);
                Command::none()
            }
            Message::Skip(f) => {
                self.player.skip(f);
                Command::none()
            }
            Message::Download(key) => self.downloader.add(key, &mut self.episode_db),
            Message::Remove(key, file_type) => todo!(),
            Message::PlayPause => self.player.play_pause(),
            Message::Podcasts(m) => self.podcasts.update(m),
        }
    }
    fn subscription(&self) -> Subscription<Self::Message> {
        use play::Track;
        use std::time::Duration;

        let mut subs = Vec::new();
        match &self.player.current {
            Track::Stream(_,_,url) => {
                let stream = play::subscribe::play(url.to_owned()).map(Message::StreamProgress);
                let time = iced::time::every(Duration::from_millis(1000/6)).map(Message::PlayBackTick);
                subs.push(stream);
                subs.push(time);
            }
            Track::File(_,_) => {
                let time = iced::time::every(Duration::from_millis(1000/6)).map(Message::PlayBackTick);
                subs.push(time);
            }
            _ => (),
        }

        subs.extend(self.downloader.subs());
        Subscription::batch(subs)
    }
    fn view(&mut self) -> Element<Self::Message> {
        let content = match self.current {
            Page::Podcasts => self.podcasts.view(), // TODO load from a cache
            Page::Episodes => self.episodes.view(),
        };
        let column = Column::new();
        let column = column.push(content);
        let column = column.push(self.player.view());
        let column = column.push(self.controls.view());
        
        iced::Container::new(column).into()
    }
    fn mode(&self) -> iced::window::Mode {
        #[cfg(features="pinephone")]
        dbg!("fullliyyyyy");
        #[cfg(features="pinephone")]
        return iced::window::Mode::Fullscreen;
        #[cfg(not(features="pinephone"))]
        return iced::window::Mode::Windowed;
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
        #[cfg(not(features="pinephone"))]
        default_text_size: 20,
        #[cfg(features="pinephone")]
        default_text_size: 1,
        antialiasing: false,
    }
}
