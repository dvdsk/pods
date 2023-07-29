use std::sync::Arc;

use tokio::task::JoinSet;
use traits::AppUpdate;
use traits::IndexSearcher;

use tokio::sync::Mutex;

use traits::ReturnTx;
use traits::Updater;

pub struct Tasks {
    set: JoinSet<()>,
    searcher: Arc<Mutex<Box<dyn IndexSearcher>>>,
}

impl Tasks {
    pub(super) fn new(searcher: Arc<Mutex<Box<dyn IndexSearcher>>>) -> Self {
        /* TODO: move searcher to presenter, as there should be one per
         * user not per running panda server/backend <dvdsk noreply@davidsk.dev> */
        Self {
            set: JoinSet::new(),
            searcher,
        }
    }

    pub fn search(&mut self, query: String, tx: Box<dyn Updater>) {
        let search = search(self.searcher.clone(), query, tx);
        self.set.spawn(search);
    }

    pub(crate) fn add_podcast(&self, podcast: traits::SearchResult, tx: Box<dyn Updater>) {
        todo!()
    }
}

// async add_podcast() {
// }

async fn search(
    searcher: Arc<Mutex<Box<dyn IndexSearcher>>>,
    query: String,
    mut awnser: Box<dyn Updater>,
) {
    let mut searcher = searcher.lock().await;
    let (val, err) = searcher.search(&query).await;
    if let Err(_) = awnser.update(AppUpdate::SearchResults(val)).await {
        tracing::debug!("Search was canceld");
    }
}
