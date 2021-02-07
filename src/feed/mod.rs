use url::Url;
use std::str::FromStr;
use eyre::WrapErr;

mod search;
use crate::database::podcasts::{EpisodeList, EpisodeInfo, Progress};
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

fn get_episode_info(items: &[rss::Item]) -> eyre::Result<Vec<EpisodeInfo>> {
    Ok(items.iter()
        .filter_map(|x| x.title())
        .map(|t| EpisodeInfo {
        title: t.to_owned(),
        progress: Progress::None,
    }).collect())
}

pub async fn add_podcast(mut pod_db: database::Podcasts, mut episode_db: database::Episodes,
    url: String) -> (String, u64) {
    let info = get_podcast_info(&url).await.unwrap();
    let title = info.title().to_owned();
    let episodes = get_episode_info(info.items()).unwrap();
    let episode_list = EpisodeList {podcast: title.clone(), items: episodes};
    let id = pod_db.add_feed(&title, &url, episode_list).unwrap();
    episode_db.add_feed(id, info);
    (title, id)
}
