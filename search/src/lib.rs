use async_trait::async_trait;
use traits::IndexSearcher;

mod apikey;
pub use apikey::{APIKEY, APISECRET};

mod applepodcasts;
mod podcastindex;
pub(crate) mod budget;
mod combiner;

#[async_trait]
pub trait SearchBackend {
    async fn search(
        &mut self,
        search_term: &str,
        ignore_budget: bool,
    ) -> Result<Vec<traits::SearchResult>, Error>;
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


#[tokio::test]
async fn find_99pi() {
    let mut searcher = SearchCombiner::default();
    let res = searcher
        .search("Soft Skills Engineering".to_owned(), false)
        .await;
    assert_eq!(res[0].title, "Soft Skills Engineering");
}
