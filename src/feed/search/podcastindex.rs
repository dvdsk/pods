use regex::Regex;
use super::{Error, ApiBudget, SearchResult, APIKEY, APISECRET, APP_USER_AGENT};

#[derive(Clone)]
pub struct Search {
    client: reqwest::Client,
    title_url: Regex,
    budget: ApiBudget,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(APP_USER_AGENT)
                .build()
                .expect("could not construct http client for podcast searching"),
            title_url: Regex::new(r#""title":"(.+?)","url":"(.+?)","originalUrl":"#).unwrap(),
            budget: ApiBudget::from(20),
        }
    }
}

impl Search {
    fn to_results(&self, text: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        for cap in self.title_url.captures_iter(text) {
            results.push(SearchResult{ 
                title: cap.get(1)
                    .expect("malformed search result")
                    .as_str().to_owned(),
                url: cap.get(2)
                    .expect("malformed search result")
                    .as_str().to_owned().replace(r#"\/"#, r#"/"#),
            });
        }
        results
    }

    async fn request(&mut self, headers: reqwest::header::HeaderMap, search_term: &str) -> Result<String, Error> {
        let text = self.client.get("https://api.podcastindex.org/api/1.0/search/byterm")
            .headers(headers)
            .timeout(std::time::Duration::from_millis(1000))
            .query(&[("q",search_term)])
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


    pub async fn search(&mut self, search_term: &str, ignore_budget: bool) -> Result<Vec<SearchResult>, Error> {
        use reqwest::header::{HeaderMap, HeaderName};
        use std::time::{SystemTime, UNIX_EPOCH};
        use sha1::{Sha1, Digest};

        if self.budget.left() <= 2 && !ignore_budget {
            return Err(Error::OutOfCalls);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH).unwrap()
            .as_secs().to_string();
        
        let hash = Sha1::new()
            .chain(&APIKEY)
            .chain(&APISECRET)
            .chain(&now)
            .finalize();
        let hash = format!("{:x}", hash);
        
        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("x-auth-date"), now.parse().unwrap());
        headers.insert(HeaderName::from_static("x-auth-key"), APIKEY.parse().unwrap());
        headers.insert(HeaderName::from_static("authorization"), hash.parse().unwrap());

        self.budget.register_call();
        let text = self.request(headers, search_term).await;
        if let Err(Error::CouldNotConnect(_)) = &text {
            self.budget.update(-1);
        }
        let results = self.to_results(&text?);
        Ok(results)
    }
}

#[test]
fn test_podcast_index(){
    use tokio::runtime::Runtime;

    let mut searcher = Search::default();
    // Create the runtime
    Runtime::new()
        .unwrap()
        .block_on(async {
            let res = searcher.search("Soft Skills Engineering").await.unwrap();
            assert_eq!(res[0].title, "Soft Skills Engineering");
            assert_eq!(res[0].url, "http://feeds.feedburner.com/SoftSkillsEngineering");
        });
}
