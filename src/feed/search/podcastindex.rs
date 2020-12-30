use eyre::{eyre, Result, WrapErr};
use regex::Regex;
use super::{SearchResult, APIKEY, APISECRET, APP_USER_AGENT};

#[derive(Clone)]
pub struct Search {
    client: reqwest::Client,
    title_url: Regex,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(APP_USER_AGENT)
                .build()
                .wrap_err("could not construct http client for podcast searching").unwrap(),
            title_url: Regex::new(r#""title":"(.+?)","url":"(.+?)","originalUrl":"#).unwrap(),
        }
    }
}

impl Search {
    fn to_results(&self, text: &str) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        for cap in self.title_url.captures_iter(text) {
            results.push(SearchResult{ 
                title: cap.get(1)
                    .ok_or_else(|| eyre!("malformed search result"))?
                    .as_str().to_owned(),
                url: cap.get(2)
                    .ok_or_else(|| eyre!("malformed search result"))?
                    .as_str().to_owned().replace(r#"\/"#, r#"/"#),
            });
        }
        Ok(results)
    }

    pub async fn search(&mut self, search_term: &str) -> Result<Vec<SearchResult>> {
        use reqwest::header::{HeaderMap, HeaderName};
        use std::time::{SystemTime, UNIX_EPOCH};
        use sha1::{Sha1, Digest};

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

        let text = self.client.get("https://api.podcastindex.org/api/1.0/search/byterm")
            .headers(headers)
            .timeout(std::time::Duration::from_millis(2000))
            .query(&[("q",search_term)])
            .send()
            .await
            .wrap_err("could not connect to 'the podcast index'")?
            .error_for_status()
            .wrap_err("server replied with error")?
            .text()
            .await
            .wrap_err("could not understand response from 'the podcast index'")?;
        let results = self.to_results(&text)?;
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
