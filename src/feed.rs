use crate::db;
use eyre::{Result, WrapErr};

// Name user agent after app
static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);

pub struct Search {
    client: reqwest::Client,
}

impl Search {
    fn new() -> Result<Self> {
        Ok(Search {
            client: reqwest::Client::builder()
                .user_agent(APP_USER_AGENT)
                .build()
                .wrap_err("could not construct http client for podcast searching")?
        })
    }

    async fn search(&mut self, search_term: &str) -> Result<()> {
        use reqwest::header::{HeaderMap, HeaderName};
        use std::time::{SystemTime, UNIX_EPOCH};
        use sha1::{Sha1, Digest};

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let now = now.as_secs().to_string();
        
        let hash = Sha1::new()
            .chain(&apikey)
            .chain(apisecret)
            .chain(&now)
            .finalize();
        let hash = format!("{:x}", hash);
        dbg!(&hash);

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("x-auth-date"), now.parse().unwrap());
        headers.insert(HeaderName::from_static("x-auth-key"), apikey.parse().unwrap());
        headers.insert(HeaderName::from_static("authorization"), hash.parse().unwrap());
        let res = self.client.get("https://api.podcastindex.org/api/1.0/search")
            .headers(headers)
            .send()
            .await
            .wrap_err("could not connect to 'the podcast index'")?;
        dbg!(res);
        Ok(())
    }
}

#[test]
fn find_99pi(){
    tokio::spawn(async {
        let mut searcher = Search::new().unwrap();
        searcher.search("99pi").await.unwrap();
    });
}
