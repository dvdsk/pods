mod iced_wrapped;
mod page;
mod database;
mod download;
mod feed;
mod play;
mod widgets;

use database::{EpisodeKey, PodcastDb, PodcastKey, Progress};
use download::Downloader;
use download::FileType;
use error_level::ErrorLevel;
use page::{Controls, Page};
use play::Player;

use iced::{executor, Application, Column, Command, Element, Settings, Subscription};
use std::collections::HashMap;

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
    None,
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

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (App, Command<Self::Message>) {
        let db = database::open().unwrap();
        let pod_db = PodcastDb::open(&db).unwrap();
        let startup = iced_wrapped::update_podcasts(pod_db.clone());
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
            Message::Back => self.current.back(),
            Message::Up => match &self.current {
                Page::Podcasts => self.podcasts.up(),
                Page::Episodes => self.episodes.up(),
            },
            Message::Down => match &self.current {
                Page::Podcasts => self.podcasts.down(),
                Page::Episodes => self.episodes.down(),
            },
            Message::ToEpisodes(podcast_id) => {
                let podcast = self.pod_db.get_podcast(podcast_id).unwrap();
                return Command::perform(download::scan_podcast_dir(podcast.title), move |set| {
                    Message::ToEpisodesFinish(set, podcast_id)
                });
            }
            Message::ToEpisodesFinish(downloaded, podcast_id) => {
                self.episodes.populate(podcast_id, downloaded);
                self.current = Page::Episodes;
            }
            Message::StreamProgress(p) => {
                use play::subscribe::Progress;
                match p {
                    Progress::ToShortError => log::warn!("stream was to short to play audio"),
                    Progress::StreamError(e) => log::error!("errored stream {}", e),
                    Progress::Started(rx) => self.player.rx = Some(rx),
                    Progress::Finished => (),
                    Progress::Advanced(p) => {
                        self.player.current.set_streampos(p);
                        if self.player.stream_ready(p) {
                            self.player.start_stream();
                        }
                    }
                }
            }
            Message::DownloadProgress(download::Progress::Error(e)) => e.log_error(),
            Message::DownloadProgress(download::Progress::Finished) => {
                log::info!("finished download");
                let podcast = self.episodes.podcast.as_ref().unwrap().clone();
                return Command::perform(
                    download::scan_podcast_dir(podcast),
                    Message::DownloadFinished,
                );
            }
            Message::DownloadProgress(_) => (),
            Message::DownloadFinished(set) => self.episodes.update_downloaded(set),
            Message::PlayBackTick(_) => {
                if let Some(pos) = self.player.should_store_pos() {
                    if let Some(info) = self.player.current.info() {
                        let progress = Progress::Listening(pos);
                        return iced_wrapped::update_episode_progress(&self.pod_db, info.id, progress);
                    }
                }
                // also used to trigger a redraw
            }
            Message::AddPodcast(url) => return iced_wrapped::add_podcast(&self.pod_db, url),
            Message::PodcastsUpdated => {
                if let Page::Episodes = self.current {
                    self.episodes.repopulate(HashMap::new());
                }
            }
            Message::Stream(key) => self.player.add_stream(key),
            Message::Play(key, file_type, pos) => self.player.add_file(key, file_type, pos),
            Message::Skip(f) => self.player.skip(f),
            Message::Download(key) => return self.downloader.add(key, &mut self.pod_db),
            Message::Remove(_key, _file_type) => todo!(),
            Message::PlayPause => return self.player.play_pause(),
            Message::SearchSubmit => return self.podcasts.search.do_search(true),
            Message::SearchInputChanged(input) => {
                return self
                    .podcasts
                    .search
                    .input_changed(self.pod_db.clone(), input)
            }
            Message::SearchResults(r) => self.podcasts.list.update_feedres(r),
            Message::AddedPodcast(title, id) => {
                self.podcasts.list.remove_feedres();
                self.podcasts.search.reset();
                self.podcasts.list.add(title, id);
            }
            Message::None => (),
        }
        Command::none()
    }
    fn subscription(&self) -> Subscription<Self::Message> {
        use play::Track;
        use std::time::Duration;

        let mut subs = Vec::new();
        match &self.player.current {
            Track::Stream(_, _, url) => {
                let stream = play::subscribe::play(url.to_owned()).map(Message::StreamProgress);
                let time =
                    iced::time::every(Duration::from_millis(1000 / 6)).map(Message::PlayBackTick);
                subs.push(stream);
                subs.push(time);
            }
            Track::File(_, _) => {
                let time =
                    iced::time::every(Duration::from_millis(1000 / 6)).map(Message::PlayBackTick);
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
        let column = Column::new()
            .push(content)
            .push(self.player.view())
            .push(self.controls.view());

        iced::Container::new(column).into()
    }
    fn mode(&self) -> iced::window::Mode {
        #[cfg(features = "pinephone")]
        return iced::window::Mode::Fullscreen;
        #[cfg(not(features = "pinephone"))]
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
        #[cfg(not(features = "pinephone"))]
        default_text_size: 20,
        #[cfg(features = "pinephone")]
        default_text_size: 1,
        antialiasing: false,
    }
}
