use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use traits::DataSub;
use traits::Feed;
use traits::IdGen;
use traits::IndexSearcher;
use traits::{AppUpdate, DataRStore, DataUpdate, DataWStore, Episode, EpisodeDetails};

use tokio::sync::Mutex;

use traits::Podcast;
use traits::Registration;
use traits::Updater;

pub struct Tasks {
    set: JoinSet<()>,
    searcher: Arc<Mutex<dyn IndexSearcher>>,
    podcast_id_gen: Box<dyn IdGen>,
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
            podcast_id_gen: db_writer.podcast_id_gen(),
            db_writer,
            db_reader,
        }
    }

    pub async fn panicked(&mut self) -> Box<dyn std::any::Any + Send + 'static> {
        loop {
            let finished = self
                .set
                .join_next()
                .await
                .expect("set is never empty since `maintain_feed` runs till the end");
            if let Err(e) = finished {
                if e.is_panic() {
                    return e.into_panic();
                }
            }
        }
    }

    pub fn search(&mut self, query: String, tx: Box<dyn Updater>) {
        let search = search(self.searcher.clone(), query, tx);
        self.set.spawn(search);
    }

    #[instrument(level = "Info", skip(self, tx))]
    pub(crate) fn add_podcast(&mut self, podcast: traits::SearchResult, tx: Box<dyn Updater>) {
        let id = self.podcast_id_gen.next();
        let podcast = Podcast::try_from_searchres(podcast, id).unwrap();
        // check for duplicates in db TODO
        self.db_writer.add_podcast(podcast.clone());
    }

    pub(crate) async fn start_maintain_feed(&mut self, feed: Box<dyn Feed>) {
        let (tx, rx) = mpsc::channel(10);
        let reg = self.db_reader.register(tx, "maintain_feed");
        let sub = self.db_reader.sub_podcasts(reg);
        let ready = Arc::new(Notify::new());
        let maintain = maintain_feed(
            rx,
            feed,
            self.db_writer.box_clone(),
            reg,
            sub,
            ready.clone(),
        );
        self.set.spawn(maintain);
        ready.notified().await
    }
}

// TODO report errors?
#[instrument(skip(rx, feed, db, _subscription))]
async fn maintain_feed(
    mut rx: mpsc::Receiver<DataUpdate>,
    feed: Box<dyn Feed>,
    mut db: Box<dyn DataWStore>,
    _registration: Registration,
    _subscription: Box<dyn DataSub>,
    ready: Arc<Notify>,
) {
    let mut known = HashSet::new();
    let mut idgen = db.episode_id_gen();
    info!("maintaining feed, ready");
    ready.notify_one();
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
            let info = feed.index(new_podcast).await.unwrap();
            let (list, details) = info
                .into_iter()
                .map(|e| {
                    let episode_id = idgen.next();
                    (
                        Episode {
                            name: e.title,
                            id: episode_id,
                        },
                        EpisodeDetails {
                            url: e.stream_url,
                            description: e.description,
                            duration: e.duration,
                            date: e.date,
                            episode_id,
                        },
                    )
                })
                .unzip();
            info!(
                "adding episode for podcast: {}, id: {}",
                new_podcast.name, new_podcast.id
            );
            db.add_episodes(new_podcast.id, list);
            db.add_episode_details(details);
        }
        known.extend(podcasts.into_iter());
        // TODO check for updates to existing podcasts
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
