use std::collections::HashSet;
use std::time::Duration;
use std::time::Instant;
use arraydeque::{ArrayDeque, Wrapping};

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

#[derive(Clone)]
struct ApiBudget {
    max_per_min: u8,
    current_per_min: u8,
    last_called: Instant,
    called: ArrayDeque<[Instant;20], Wrapping>,
}

impl ApiBudget {
    fn from(max_per_min: u8) -> Self {
        Self {
            max_per_min,
            current_per_min: max_per_min,
            last_called: Instant::now(),
            called: ArrayDeque::new(), 
        }
    }
    /// modify the apibudget depending on how the last api call went
    fn update(&mut self, success: i8) {
        let current = self.current_per_min as f32;
        let new = (0.8f32 * current + success as f32) as u8;
        let new = new.min(1);
        let new = new.max(self.max_per_min);
        self.current_per_min = new;
    }
    fn calls_in_last_minute(&self) -> usize {
        self.called.iter()
            .take_while(|t| t.elapsed() < Duration::from_secs(61))
            .count()
    }
    pub fn left(&self) -> u8 {
        self.current_per_min.saturating_sub(self.calls_in_last_minute() as u8)
    }
    pub fn register_call(&mut self) {
        self.called.push_front(Instant::now());
    }
}

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
    -> Vec<SearchResult> {
        let search_a = self.apple_podcasts.search(&search_term, ignore_budget);
        let search_b = self.podcast_index.search(&search_term, ignore_budget);

        let (res_a, res_b) = tokio::join!(search_a, search_b);
        match (res_a, res_b) {
            (Err(_), Err(e2)) => {return Vec::new()},//TODO log the errs
            (Err(_), Ok(b)) => {return b}, //TODO log the errs
            (Ok(a), Err(_)) => {return a},
            (Ok(mut a), Ok(mut b)) => {
                let mut result = HashSet::new();
                for res in a.drain(..).chain(b.drain(..)){
                    result.insert(res);
                }
                let result = result.drain().collect();
                result
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
