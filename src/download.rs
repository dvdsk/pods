use reqwest::Client;
use crate::database::episodes::Key as EpisodeKey;
use crate::{database, Message};
use iced::Subscription;
use std::path::PathBuf;

mod subscribe;
pub use subscribe::Progress;

type Url = String;
#[derive(Clone)]
pub struct Download {
    url: Url,
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
        let dl = Download {path: episode.temp_file_path(), url: episode.stream_url};
        self.downloading.push(dl);
        iced::Command::none()
    }
    // pub fn subs(&mut self) -> impl Iterator<Item=Subscription<subscribe::Progress>> {
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

