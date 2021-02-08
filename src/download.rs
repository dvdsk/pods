use crate::database::{EpisodeKey, EpisodeExt};
use crate::{database, Message};
use iced::Subscription;
use std::path::PathBuf;

mod subscribe;
pub use subscribe::Progress;

#[derive(Clone, Debug)]
pub struct Download {
    url: reqwest::Url,
    path: PathBuf,
}

#[derive(Default)]
pub struct Downloader {
    downloading: Vec<Download>,
}

impl Downloader {
    pub fn add(&mut self, id: EpisodeKey, db: &mut database::PodcastDb) -> iced::Command<Message> {
        let episode = db.get_episode_ext(id)
            .expect("item should be in database when we start downloading");
        let url = reqwest::Url::parse(&episode.stream_url).expect("url should be valid here");
        let extension = url.path().rsplitn(2, ".").next().expect("there has to be a file extension");
        let mut path = base_file_path(&episode);
        path.set_extension(&format!("{}.part", extension));
        let dl = Download {path, url};
        self.downloading.push(dl);
        iced::Command::none()
    }
    pub fn subs(&self) -> Vec<Subscription<Message>> {
        const N: usize = 2; //number of downloads to handle simultaneously

        self.downloading
            .iter()
            .cloned()
            .take(N)
            .map(Self::subscribe)
            .collect()
    }
    fn subscribe(item: Download) -> iced::Subscription<Message> {
        iced::Subscription::from_recipe(item).map(Message::DownloadProgress)
    }
}

/// path to file without any extension
pub fn base_file_path(episode: &EpisodeExt) -> PathBuf {
    use directories::UserDirs;
    let user_dirs = UserDirs::new()
        .expect("can not download if the user has no home directory");
    let mut dl_dir = user_dirs.download_dir()
        .expect("need a download folder to be able to download")
        .to_owned();
    dl_dir.push(env!("CARGO_BIN_NAME"));
    dl_dir.push(&episode.podcast);
    dl_dir.push(&episode.title);
    dl_dir
}
