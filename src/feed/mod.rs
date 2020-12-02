use url::Url;
use std::str::FromStr;
use eyre::WrapErr;

mod search;
use crate::database::podcasts::{EpisodeList, EpisodeInfo};
use crate::database;
pub use search::{Search, SearchResult};

pub fn valid_url(s: &str) -> bool {
    if let Ok(url) = Url::parse(s) {
        url.scheme() == "http" || url.scheme() == "https"
    } else {
        false
    }
}

async fn get_podcast_info(url: &str) -> eyre::Result<rss::Channel> {
    let feed_text = reqwest::get(url)
        .await
        .wrap_err("could not connect to podcast feed")?
        .error_for_status()
        .wrap_err("feed server returned error")?
        .text()
        .await
        .wrap_err("could not download body")?;

    let channel = rss::Channel::from_str(&feed_text)
        .wrap_err_with(|| format!("can not parse feed body as rss, text: {}", url))?;
    Ok(channel)
}

async fn get_episode_info(items: &[rss::Item]) -> eyre::Result<EpisodeList> {
    Ok(items.iter()
        .filter_map(|x| x.title())
        .map(|t| EpisodeInfo {
        title: t.to_owned(),
        listend: false,
    }).collect())
}

//TODO let this return episodes? could then directly draw the episodes screen
pub async fn add_podcast(mut db: database::Podcasts, url: String) -> (String, u64) {
    let info = get_podcast_info(&url).await.unwrap();
    let episodes = get_episode_info(info.items());
    let id = db.add_to_podcastlist(info.title(), &url).unwrap();
    db.add_to_episodelist(id, episodes.await.unwrap()).unwrap();
    (info.title().to_owned(), id)
}