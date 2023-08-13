use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::debug;
use traits::AppUpdate;
use traits::DataRStore;
use traits::DataUpdate;
use traits::DataWStore;
use traits::Feed;
use traits::IndexSearcher;

use tokio::sync::Mutex;

use traits::Podcast;
use traits::Registration;
use traits::Updater;

pub struct Tasks {
    set: JoinSet<()>,
    searcher: Arc<Mutex<dyn IndexSearcher>>,
    db_writer: Box<dyn DataWStore>,
    db_reader: Box<dyn DataRStore>,
}

impl Tasks {
    pub(super) fn new(
        searcher: Arc<Mutex<dyn IndexSearcher>>,
        db_writer: Box<dyn DataWStore>,
        db_reader: Box<dyn DataRStore>,
    ) -> Self {
        /* TODO: move searcher to presenter, as there should be one per
         * user not per running panda server/backend <dvdsk noreply@davidsk.dev> */
        Self {
            set: JoinSet::new(),
            searcher,
            db_writer,
            db_reader,
        }
    }

    pub fn search(&mut self, query: String, tx: Box<dyn Updater>) {
        let search = search(self.searcher.clone(), query, tx);
        self.set.spawn(search);
    }

    pub(crate) fn add_podcast(&mut self, podcast: traits::SearchResult, tx: Box<dyn Updater>) {
        let id = 0;
        let podcast = Podcast::try_from_searchres(podcast, id).unwrap();
        self.db_writer.add_podcast(podcast.clone());
    }

    pub(crate) fn maintain_feed(&mut self, feed: Box<dyn Feed>) {
        let (tx, rx) = mpsc::channel(10);
        let reg = self.db_reader.register(Box::new(tx));
        self.db_reader.sub_podcasts(reg);
        let maintain = maintain_feed(rx, feed, self.db_writer.box_clone(), reg);
        self.set.spawn(maintain);
    }
}

// TODO make feed task that responds to new data
async fn maintain_feed(
    mut rx: mpsc::Receiver<DataUpdate>,
    feed: Box<dyn Feed>,
    mut db: Box<dyn DataWStore>,
    _registration: Registration,
) {
    let mut known = HashSet::new();
    loop {
        let Some(update) = rx.recv().await else {
            debug!("maintain feed stopping");
            break;
        };
        let DataUpdate::Podcasts { podcasts } = update else {
            panic!("maintain feed recieved update it is not subscribed too");
        };

        let podcasts = HashSet::from_iter(podcasts);
        for new_podcast in podcasts.difference(&known) {
            let episodes = feed.index(new_podcast).await;
            db.add_episodes(&new_podcast, episodes);
        }
        known.extend(podcasts.into_iter());
    }
}

async fn search(
    searcher: Arc<Mutex<dyn IndexSearcher>>,
    query: String,
    mut awnser: Box<dyn Updater>,
) {
    let mut searcher = searcher.lock().await;
    let (val, err) = searcher.search(&query).await;
    if let Err(_) = awnser.update(AppUpdate::SearchResults(val)).await {
        tracing::debug!("Search was canceld");
    }
}
