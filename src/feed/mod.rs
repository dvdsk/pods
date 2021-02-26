use eyre::WrapErr;
use std::str::FromStr;
use url::Url;

mod search;
use crate::database;
use crate::database::{Date, EpisodeExt, Podcast, PodcastKey};
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

fn get_episode_info(items: &[rss::Item], podcast_title: &str) -> Result<Vec<EpisodeExt>, Error> {
    let list: Result<Vec<EpisodeExt>, _> = items
        .iter()
        .map(|i| to_episode_ext(i, podcast_title))
        .collect();
    list
}

pub async fn add_podcast(pod_db: database::PodcastDb, url: String) -> (String, PodcastKey) {
    let info = get_podcast_info(&url).await.unwrap();

    let podcast = Podcast::from_url(&info, url);
    pod_db.add_podcast(&podcast).unwrap();

    let episodes = get_episode_info(info.items(), &podcast.title).unwrap();
    pod_db
        .update_episodes(podcast.title.as_str(), episodes)
        .unwrap();

    (podcast.title.clone(), PodcastKey::from(podcast.title))
}

fn url_from_extensions(item: &rss::Item) -> Option<String> {
    let media = item.extensions().get("media")?;
    let content = media.get("content")?;
    let extention = content.first()?;
    if extention.name() != "media:content" {
        return None;
    }
    extention.attrs().get("url").cloned()
}

fn duration_from_extensions(item: &rss::Item) -> Option<f32> {
    let media = item.extensions().get("media")?;
    let content = media.get("content")?;
    let extention = content.first()?;
    if extention.name() != "media:content" {
        return None;
    }
    extention
        .attrs()
        .get("duration")
        .map(|u| u.parse().ok())
        .flatten()
}

fn parse_itunes_duration(duration: &str) -> Option<f32> {
    let mut parts = duration.rsplitn(3, ':');
    let seconds: f32 = parts.next()?.parse().ok()?;
    let minutes: f32 = parts.next()?.parse().ok()?;
    let hours: f32 = parts.next().unwrap_or("0").parse().ok()?;
    let seconds = seconds + 60. * (minutes + 60. * hours);
    Some(seconds)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No stream for podcast episode")]
    MissingStreamUrl,
    #[error("No duration for podcast episode")]
    MissingDuration,
    #[error("No title for podcast episode")]
    MissingEpisodeTitle,
}

fn to_episode_ext(item: &rss::Item, podcast_title: &str) -> Result<EpisodeExt, Error> {
    //try to get the url from the description of the media object
    let stream_url = item.enclosure().map(|encl| encl.url().to_owned());

    //try to get the url and duration possible extensions
    let stream_url = stream_url.or_else(|| url_from_extensions(item));
    let duration = duration_from_extensions(item);

    //try to get duration from any included itunes extensions
    let duration = duration.or_else(|| {
        item.itunes_ext()
            .map(|ext| ext.duration().map(parse_itunes_duration).flatten())
            .flatten()
    });

    let stream_url = stream_url.ok_or(Error::MissingStreamUrl)?;
    let duration = duration.ok_or(Error::MissingDuration)?;
    let title = item.title().ok_or(Error::MissingEpisodeTitle)?;
    let podcast = podcast_title.to_owned();

    Ok(EpisodeExt {
        stream_url,
        duration,
        title: title.to_owned(),
        podcast,
        date: Date::from_item(item),
    })
}
