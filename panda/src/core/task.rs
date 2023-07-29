use std::sync::Arc;

use tokio::task::JoinSet;
use traits::AppUpdate;
use traits::IndexSearcher;

use tokio::sync::Mutex;

use traits::ReturnTx;

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

    pub fn search(&mut self, query: String, awnser: ReturnTx) {
        let search = search(self.searcher.clone(), query, awnser);
        self.set.spawn(search);
    }
}

async fn search(searcher: Arc<Mutex<Box<dyn IndexSearcher>>>, query: String, awnser: ReturnTx) {
    let mut searcher = searcher.lock().await;
    let (val, err) = searcher.search(&query).await;
    if let Err(_) = awnser.send(AppUpdate::SearchResults(val)) {
        tracing::debug!("Search was canceld");
    }
}
