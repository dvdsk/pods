use std::convert::TryFrom;
use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use eyre::eyre;

fn url_from_extensions(item: &rss::Item) -> Option<String> {
    let media = item.extensions().get("media")?;
    let content = media.get("content")?;
    let extention = content.first()?;
    if extention.name() != "media:content" { return None }
    extention.attrs().get("url").map(|u| u.clone())
}

fn duration_from_extensions(item: &rss::Item) -> Option<f32> {
    let media = item.extensions().get("media")?;
    let content = media.get("content")?;
    let extention = content.first()?;
    if extention.name() != "media:content" { return None }
    extention.attrs().get("duration").map(|u| u.parse().ok()).flatten()
}

fn parse_itunes_duration(duration: &str) -> Option<f32> {
    let mut parts = duration.rsplitn(3, ":");
    let seconds: f32 = parts.next()?.parse().ok()?;
    let minutes: f32 = parts.next()?.parse().ok()?;
    let hours: f32 = parts.next().unwrap_or("0").parse().ok()?;
    let seconds = seconds + 60.*(minutes + 60.*hours);
    Some(seconds)
}

fn try_from(item: &rss::Item, podcast_title: &str) -> Result<EpisodeExt, Self::Error> {
    //try to get the url from the description of the media object
    let stream_url = item.enclosure().map(|encl| encl.url().to_owned());

    //try to get the url and duration possible extensions
    let stream_url = stream_url.or(url_from_extensions(item));
    let duration = duration_from_extensions(item);

    //try to get duration from any included itunes extensions
    let duration = duration.or(item.itunes_ext()
        .map(|ext| ext.duration()
            .map(parse_itunes_duration).flatten()
        ).flatten());

    let stream_url = stream_url.ok_or(eyre!("no link for feed item: {:?}", item))?;
    let duration = duration.ok_or(eyre!("no duration known for item: {:?}", item))?;
    let title = item.title().ok_or(eyre!("episode should have a title: {:?}", item))?.to_owned();
    let podcast = podcast_title.to_owned();

    Ok(EpisodeExt {
        stream_url,
        duration,
        title,
        podcast,
    })
}
