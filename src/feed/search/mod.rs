use std::collections::HashSet;

use eyre::Result;

mod apikey;
pub use apikey::{APIKEY, APISECRET};

mod podcastindex;
mod applepodcasts;

// Name user agent after app
static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);

#[derive(Default, Clone)]
pub struct Search {
    apple_podcasts: applepodcasts::Search,
    podcast_index: podcastindex::Search,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
}

impl Search {
    pub async fn search(&mut self, search_term: String, ignore_budget: bool) 
    -> Result<Vec<SearchResult>> {
        let search_a = self.apple_podcasts.search(&search_term, ignore_budget);
        let search_b = self.podcast_index.search(&search_term);

        let (res_a, res_b) = tokio::join!(search_a, search_b);
        match (res_a, res_b) {
            (Err(e), Err(_)) => return Err(e),//TODO log the errs
            (Err(e), Ok(b)) => {dbg!(e); return Ok(b)}, //TODO log the errs
            (Ok(a), Err(e)) => {dbg!(e); return Ok(a)},
            (Ok(mut a), Ok(mut b)) => {
                let mut result = HashSet::new();
                for res in a.drain(..).chain(b.drain(..)){
                    result.insert(res);
                }
                let result = result.drain().collect();
                Ok(result)
            }
        }
    }
}

#[test]
fn find_99pi(){
    use tokio::runtime::Runtime;

    let mut searcher = Search::default();
    // Create the runtime
    Runtime::new()
        .unwrap()
        .block_on(async {
            let res = searcher.search("Soft Skills Engineering".to_owned(), false).await.unwrap();
            assert_eq!(res[0].title, "Soft Skills Engineering");
        });
}
