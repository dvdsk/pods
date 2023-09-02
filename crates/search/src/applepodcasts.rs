use crate::budget::ApiBudget;
use traits::SearchResult;
use super::{Error, APP_USER_AGENT};
use async_trait::async_trait;
use regex::Regex;

#[derive(Clone, Debug)]
pub struct Search {
    client: reqwest::Client,
    title: Regex,
    url: Regex,
    budget: ApiBudget,
}

#[async_trait]
impl crate::SearchBackend for Search {
    async fn search(
        &mut self,
        search_term: &str,
        ignore_budget: bool,
    ) -> Result<Vec<SearchResult>, Error> {
        if self.budget.left() <= 2 && !ignore_budget {
            return Err(Error::OutOfCalls);
        }

        self.budget.register_call();
        let text = self.request(search_term).await;

        if let Err(Error::CouldNotConnect(_)) = &text {
            self.budget.update(-1);
        }
        let results = self.to_results(&text?);
        Ok(results)
    }
}

impl Default for Search {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(APP_USER_AGENT)
                .build()
                .expect("could not construct http client for podcast searching"),
            title: Regex::new(r#"collectionName":"(.+?)""#).unwrap(),
            url: Regex::new(r#"feedUrl":"(.+?)""#).unwrap(),
            budget: ApiBudget::from(20),
        }
    }
}

impl Search {
    pub fn to_results(&self, text: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        for (cap1, cap2) in self
            .title
            .captures_iter(text)
            .zip(self.url.captures_iter(text))
        {
            results.push(SearchResult {
                title: cap1
                    .get(1)
                    .expect("malformed search result")
                    .as_str()
                    .to_owned(),
                url: cap2
                    .get(1)
                    .expect("malformed search result")
                    .as_str()
                    .to_owned(),
            });
        }
        results
    }

    async fn request(&mut self, search_term: &str) -> Result<String, Error> {
        let text = self
            .client
            .get("https://itunes.apple.com/search")
            .timeout(std::time::Duration::from_millis(5000))
            .query(&[("entity", "podcast")])
            .query(&[("term", search_term)])
            .query(&[("limit", 25)])
            .query(&[("explicit", "Yes")])
            .send()
            .await
            .map_err(Error::CouldNotConnect)?
            .error_for_status()
            .map_err(Error::HttpError)?
            .text()
            .await
            .map_err(Error::NoText)?;
        Ok(text)
    }
}

#[tokio::test]
async fn test_apple_podcasts() {
    use crate::SearchBackend;
    let mut searcher = Search::default();
    let res = searcher.search("Soft Skills", true).await.unwrap();
    assert_eq!(res[0].title, "Soft Skills Engineering");
    assert_eq!(res[0].url, "https://softskills.audio/feed.xml");
}
