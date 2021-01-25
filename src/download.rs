use crate::database::episodes::Key as EpisodeKey;
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
    pub fn add(&mut self, key: EpisodeKey, db: &mut database::Episodes) -> iced::Command<Message> {
        let episode = db.get(key)
            .expect("item should be in database when we start downloading");
        let url = reqwest::Url::parse(&episode.stream_url).expect("url should be valid here");
        let extension = url.path().rsplitn(2, ".").next().expect("there has to be a file extension");
        let mut path = episode.base_file_path();
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

