use std::time::Duration;
use std::time::Instant;
use arraydeque::{ArrayDeque, Wrapping};

use eyre::{eyre, Result, WrapErr};
use regex::Regex;
use super::{SearchResult, APP_USER_AGENT};

#[derive(Clone)]
struct ApiBudget {
    last_called: Instant,
    called: ArrayDeque<[Instant;20], Wrapping>,
}
impl Default for ApiBudget {
    fn default() -> Self {
        Self {
            last_called: Instant::now(),
            called: ArrayDeque::new(), 
        }
    }
}

impl ApiBudget {
    const CALL_PER_MINUTE: u8 = 20;
    fn calls_in_last_minute(&self) -> usize {
        self.called.iter()
            .take_while(|t| t.elapsed() < Duration::from_secs(61))
            .count()
    }
    pub fn left(&self) -> u8 {
        Self::CALL_PER_MINUTE.saturating_sub(self.calls_in_last_minute() as u8)
    }
    pub fn register_call(&mut self) {
        self.called.push_front(Instant::now());
    }
}

#[derive(Clone)]
pub struct Search {
    client: reqwest::Client,
    title: Regex,
    url: Regex,
    budget: ApiBudget,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(APP_USER_AGENT)
                .build()
                .wrap_err("could not construct http client for podcast searching").unwrap(),
            title: Regex::new(r#"collectionName":"(.+?)""#).unwrap(),
            url: Regex::new(r#"feedUrl":"(.+?)""#).unwrap(),
            budget: ApiBudget::default(),
        }
    }
}

impl Search {
    pub fn api_calls_left(&self) -> u8 {
        self.budget.left()
    }
    pub fn to_results(&self, text: &str) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        for (cap1, cap2) in self.title.captures_iter(text)
            .zip(self.url.captures_iter(text)){

            results.push(SearchResult {
                title: cap1.get(1)
                    .ok_or_else(|| eyre!("malformed search result"))?
                    .as_str().to_owned(),
                url: cap2.get(2)
                    .ok_or_else(|| eyre!("malformed search result"))?
                    .as_str().to_owned(),
                });
            }
        Ok(results) 
    }

    pub async fn search(&mut self, search_term: &str, ignore_budget: bool)
        -> Result<Vec<SearchResult>> {
        
        if self.budget.left() <= 2 {
            return Err(eyre!("over api budget"));
        }

        self.budget.register_call();
        let text = self.client.get("https://itunes.apple.com/search")
            .timeout(std::time::Duration::from_millis(2000))
            .query(&[("entity","podcast")])
            .query(&[("term",search_term)])
            .query(&[("limit",25)])
            .query(&[("explicit","Yes")])
            .send()
            .await
            .wrap_err("could not connect to apple podcasts")?
            .text()
            .await
            .wrap_err("could not understand apple podcast reply")?;

        let results = self.to_results(&text)?;
        Ok(results)
    }
}

#[test]
fn test_apple_podcasts(){
    use tokio::runtime::Runtime;

    let mut searcher = Search::default();
    // Create the runtime
    Runtime::new()
        .unwrap()
        .block_on(async {
            let res = searcher.search("Soft Skills").await.unwrap();
            assert_eq!(res[0].title, "Soft Skills Engineering");
        });
}
