use crate::database::{self, EpisodeKey, PodcastDb, Progress};
use crate::feed;
use crate::Message;

use iced::Command;

pub fn update_podcasts(pod_db: PodcastDb) -> Command<Message> {
    async fn update(pod_db: PodcastDb) {
        pod_db.update_podcasts().await.unwrap();
    }

    Command::perform(
        update(pod_db),
        |_| Message::PodcastsUpdated,
    )
}

pub fn update_episode_progress(db: &PodcastDb, id: EpisodeKey, progress: Progress) -> Command<Message> {
    async fn update(db: PodcastDb, id: EpisodeKey, progress: Progress) {
        db.update_episode_progress(id, progress).await;
    }

    let db = db.clone();
    Command::perform(update(db, id, progress), |_| Message::None)
}

pub fn add_podcast(pod_db: &PodcastDb, url: String) -> Command<Message> {
    let pod_db = pod_db.clone();
    Command::perform(
        feed::add_podcast(pod_db, url), 
        |res| match res {
            Ok((title, id)) => Message::AddedPodcast(title,id),
            Err(feed::Error::DatabaseError(database::Error::PodcastAlreadyAdded)) => Message::None,
            Err(e) => panic!(e),
        })
}
