use crate::database::{EpisodeExt, EpisodeKey};
use crate::{database, Message};
use iced::Subscription;
use std::collections::HashMap;
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
        let episode = db
            .get_episode_ext(id)
            .expect("item should be in database when we start downloading");
        let url = reqwest::Url::parse(&episode.stream_url).expect("url should be valid here");
        let extension = url
            .path()
            .rsplitn(2, '.')
            .next()
            .expect("there has to be a file extension");
        let mut path = base_file_path(&episode);
        path.set_extension(&format!("{}.part", extension));
        let dl = Download { path, url };
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
    let user_dirs = UserDirs::new().expect("can not download if the user has no home directory");
    let mut dl_dir = user_dirs
        .download_dir()
        .expect("need a download folder to be able to download")
        .to_owned();
    dl_dir.push(env!("CARGO_BIN_NAME"));
    dl_dir.push(&episode.podcast);
    dl_dir.push(&episode.title);
    dl_dir
}

#[derive(Debug, Clone, Copy)]
pub enum FileType {
    Mp3,
}

impl FileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Mp3 => "mp3",
        }
    }
}

pub fn hash(string: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;

    let mut hasher = DefaultHasher::new();
    string.hash(&mut hasher);
    hasher.finish()
}

pub async fn scan_podcast_dir(podcast: impl AsRef<str>) -> HashMap<u64, FileType> {
    use directories::UserDirs;
    use tokio::fs;

    let user_dirs = UserDirs::new().expect("can not download if the user has no home directory");
    let mut dir = user_dirs
        .download_dir()
        .expect("need a download folder to be able to download")
        .to_owned();
    dir.push(env!("CARGO_BIN_NAME"));
    dir.push(podcast.as_ref());

    let mut set = HashMap::new();
    let entries = fs::read_dir(dir).await;
    if entries.is_err() {
        return set;
    }
    let mut entries = entries.unwrap();

    while let Some(entry) = entries.next_entry().await.unwrap() {
        let relative_path = entry.file_name();
        let relative_path = relative_path.to_str().unwrap();
        if let Some(name) = relative_path.strip_suffix(".mp3")
        // .or_else(relative_path.strip_suffix(".other")
        {
            log::trace!("found on disk episode: \"{}\"", name);
            set.insert(hash(name), FileType::Mp3);
        }
    }
    set
}
