use std::str::FromStr;
use std::time::Duration;
use url::Url;

// pub use search::{Search, SearchResult};
use async_trait::async_trait;
use traits::{Date, EpisodeInfo, Podcast};

#[derive(Debug, Clone)]
pub struct Feed {}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not connect to podcast feed, details: {0:?}")]
    CouldNotConnect(reqwest::Error),
    #[error("Feed server returned error, details: {0:?}")]
    FeedServerError(reqwest::Error),
    #[error("Could not download body, details: {0:?}")]
    FaildToDownload(reqwest::Error),
    #[error("Can not parse feed body as rss, text: {0}")]
    ParsingRss(rss::Error),
    #[error("No stream for podcast episode")]
    MissingStreamUrl,
    #[error("No duration for podcast episode")]
    MissingDuration,
    #[error("No title for podcast episode")]
    MissingEpisodeTitle,
    #[error("The url: {url} is not valid because: {error}")]
    InvalidStreamUrl {
        url: String,
        error: url::ParseError,
    },
    #[error("No description for podcast episode")]
    MissingDescription,
}

impl Feed {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl traits::Feed for Feed {
    async fn index(&self, podcast: &Podcast) -> Result<Vec<EpisodeInfo>, Box<dyn traits::Error>> {
        let info = get_podcast_info(&podcast.feed).await?;
        let result: Result<Vec<_>, _> = info.items.iter().map(|i| to_episode_ext(i)).collect();
        result
            .map_err(Box::new)
            .map_err(|e| e as Box<dyn traits::Error>)
    }

    fn box_clone(&self) -> Box<dyn traits::Feed> {
        Box::new(self.clone())
    }
}

async fn get_podcast_info(feed: &Url) -> Result<rss::Channel, Error> {
    let feed_text = reqwest::get(dbg!(feed.as_str()))
        .await
        .map_err(Error::CouldNotConnect)?
        .error_for_status()
        .map_err(Error::FeedServerError)?
        .text()
        .await
        .map_err(Error::FaildToDownload)?;

    let channel = rss::Channel::from_str(&feed_text).map_err(Error::ParsingRss)?;
    Ok(channel)
}

fn to_episode_ext(item: &rss::Item) -> Result<EpisodeInfo, Error> {
    let stream_url = url_from_enclosure(item)
        .or_else(|| url_from_extensions(item))
        .ok_or(Error::MissingStreamUrl)?;
    let stream_url = url::Url::parse(stream_url).map_err(|why| Error::InvalidStreamUrl {
        url: stream_url.to_string(),
        error: why,
    })?;

    let duration = duration_from_dur_ext(item)
        .or_else(|| duration_from_itunes_ext(item))
        .map(Duration::from_secs_f32)
        .ok_or(Error::MissingDuration)?;

    let title = item.title().ok_or(Error::MissingEpisodeTitle)?;
    let description = item.description().ok_or(Error::MissingDescription)?;

    Ok(EpisodeInfo {
        stream_url,
        duration,
        title: title.to_owned(),
        date: parse_date(item),
        description: description.to_owned(),
    })
}

pub fn parse_date(item: &rss::Item) -> traits::Date {
    use chrono::{DateTime, Utc};

    let pub_date = item
        .pub_date()
        .map(DateTime::parse_from_rfc2822)
        .map(Result::ok)
        .flatten()
        .map(DateTime::from); // convert to Utc
    match pub_date {
        Some(date) => Date::Publication(date),
        None => Date::Added(Utc::now()),
    }
}

fn url_from_enclosure(item: &rss::Item) -> Option<&str> {
    item.enclosure().map(|encl| encl.url())
}

fn url_from_extensions(item: &rss::Item) -> Option<&str> {
    let media = item.extensions().get("media")?;
    let content = media.get("content")?;
    let extention = content.first()?;
    if extention.name() != "media:content" {
        return None;
    }
    extention.attrs().get("url").map(String::as_str)
}

fn duration_from_itunes_ext(item: &rss::Item) -> Option<f32> {
    item.itunes_ext()?.duration().map(parse_itunes_duration)?
}

fn duration_from_dur_ext(item: &rss::Item) -> Option<f32> {
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
    Some(seconds + 60. * (minutes + 60. * hours))
}
