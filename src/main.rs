mod page;
mod database;
mod feed;
mod play;
mod download;

use download::Downloader;
use page::{Page, Controls};
use play::Player;
use error_level::ErrorLevel;
use database::{EpisodeKey, PodcastKey, PodcastDb, Progress};
use download::FileType;

use std::collections::HashMap;
use iced::{executor, Application, Command, Element, Column, Settings, Subscription};

#[derive(Clone, Debug)]
pub enum Message {
    ToEpisodes(PodcastKey),
    ToEpisodesFinish(HashMap<u64, FileType>, PodcastKey),
    PlayBackTick(std::time::Instant),
    Stream(EpisodeKey),
    Play(EpisodeKey, FileType, f32),
    Download(EpisodeKey),
    Remove(EpisodeKey, FileType),
    Back,
    Up,
    Down,
    PlayPause,
    AddPodcast(String),
    PodcastsUpdated,
    StreamProgress(play::subscribe::Progress),
    DownloadProgress(download::Progress),
    DownloadFinished(HashMap<u64, FileType>),
    Skip(f32),
    SearchSubmit,
    SearchInputChanged(String),
    SearchResults(Vec<feed::SearchResult>),
    AddedPodcast(String, PodcastKey),
}

pub struct App {
    current: Page,
    podcasts: page::Podcasts,
    episodes: page::Episodes,
    downloader: Downloader,
    player: Player,
    controls: Controls, //Should only be needed on desktop platforms
    pod_db: PodcastDb,
}

fn update_podcasts(pod_db: PodcastDb) -> Command<Message> {
    async fn update(pod_db: PodcastDb) {
        pod_db.update_podcasts().await.unwrap();
    }

    Command::perform(
        update(pod_db),
        |_| Message::PodcastsUpdated,
    )
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (App, Command<Self::Message>) {
        let db = database::open().unwrap();
        let pod_db = PodcastDb::open(&db).unwrap();
        let startup = update_podcasts(pod_db.clone());
        (App {
            podcasts: page::Podcasts::from_db(pod_db.clone()),
            episodes: page::Episodes::from_db(pod_db.clone()), 
            current: Page::Podcasts,
            player: Player::from_db(pod_db.clone()), 
            downloader: Downloader::default(),
            controls: Controls::default(),
            pod_db,
        }, startup)
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
                let podcast = self.pod_db.get_podcast(podcast_id).unwrap();
                Command::perform(
                    download::scan_podcast_dir(podcast.title),
                    move |set| Message::ToEpisodesFinish(set, podcast_id),
                )
            }
            Message::ToEpisodesFinish(downloaded, podcast_id) => {
                self.episodes.populate(podcast_id, downloaded);
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
                            download::scan_podcast_dir(podcast),
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
                if let Some(pos) = self.player.should_store_pos() {
                    if let Some(info) = self.player.current.info() {
                        let progress = Progress::Listening(pos);
                        self.pod_db.update_episode_progress(info.id, progress);
                    }
                }
                // also used to trigger a redraw
                Command::none()
            }
            Message::AddPodcast(url) => {
                let pod_db = self.pod_db.clone();
                Command::perform(
                    feed::add_podcast(pod_db, url), 
                    |(title, id)| Message::AddedPodcast(title,id))
            }
            Message::PodcastsUpdated => {
                if let Page::Episodes = self.current {
                    self.episodes.repopulate(HashMap::new());
                }
                Command::none()
            }
            Message::Stream(key) => {
                self.player.add_stream(key);
                Command::none()
            }
            Message::Play(key, file_type, pos) => {
                self.player.add_file(key, file_type, pos);
                Command::none()
            }
            Message::Skip(f) => {
                self.player.skip(f);
                Command::none()
            }
            Message::Download(key) => self.downloader.add(key, &mut self.pod_db),
            Message::Remove(key, file_type) => todo!(),
            Message::PlayPause => self.player.play_pause(),
            Message::SearchSubmit => self
                .podcasts
                .search
                .do_search(true),
            Message::SearchInputChanged(input) => self
                .podcasts
                .search
                .input_changed(self.pod_db.clone(), input),
            Message::SearchResults(r) => {
                self.podcasts.list.update_feedres(r);
                Command::none()
            }
            Message::AddedPodcast(title,id) => {
                self.podcasts.list.remove_feedres();
                self.podcasts.search.reset();
                self.podcasts.list.add(title, id);
                Command::none()
            }
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
        subs.push(play::handle_media_keys());
        subs.extend(self.downloader.subs());
        Subscription::batch(subs)
    }
    fn view(&mut self) -> Element<Self::Message> {
        let content = match self.current {
            Page::Podcasts => self.podcasts.view(),
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
    if std::path::Path::new("log4rs.yml").exists() {
        log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    }
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
