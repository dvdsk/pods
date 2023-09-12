use core::fmt;

use async_trait::async_trait;

mod apikey;
pub use apikey::{APIKEY, APISECRET};

mod applepodcasts;
pub(crate) mod budget;
mod combiner;
mod podcastindex;

#[async_trait]
pub trait SearchBackend: fmt::Debug {
    async fn search(
        &mut self,
        search_term: &str,
        ignore_budget: bool,
    ) -> Result<Vec<traits::SearchResult>, Error>;
}

pub fn new() -> combiner::Searcher {
    combiner::Searcher {
        started: std::time::Instant::now(),
        backends: vec![
            Box::new(applepodcasts::Search::default()),
            Box::new(podcastindex::Search::default()),
        ],
    }
}

// Name user agent after app
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not connect to apple podcasts, error: {0}")]
    CouldNotConnect(reqwest::Error),
    #[error("server replied with error: {0}")]
    HttpError(reqwest::Error),
    #[error("server reply did not contain text: {0}")]
    NoText(reqwest::Error),
    #[error("no more api calls left for now")]
    OutOfCalls,
}

#[cfg(test)]
mod tests {
    use super::*;
    use traits::IndexSearcher;

    #[tokio::test]
    async fn find_99pi() {
        let mut searcher = new();
        let res = searcher.search("Soft Skills Engineering").await;
        assert_eq!(res.0.first().unwrap().title, "Soft Skills Engineering");
    }
}
