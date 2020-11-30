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

async fn get_episode_info(url: &str) -> eyre::Result<EpisodeList> {
    let feed_text = reqwest::get(url)
        .await
        .wrap_err("could not connect to podcast feed")?
        .text()
        .await
        .wrap_err("could not download body")?;

    let channel = rss::Channel::from_str(&feed_text)
        .wrap_err("can not parse feed body as rss")?;

    let list = channel.items().iter()
        .filter_map(|x| x.title())
        .map(|t| EpisodeInfo {
        title: t.to_owned(),
        listend: false,
    }).collect();

    Ok(list)
}

//TODO let this return episodes? could then directly draw the episodes screen
pub async fn add_podcast(db: &mut database::Podcasts, title: &str, url: &str) -> eyre::Result<()> {
    let episodes = get_episode_info(url);
    let id = db.add_to_podcastlist(title, url)?;
    db.add_to_episodelist(id, episodes.await?)?;
    Ok(())
}
