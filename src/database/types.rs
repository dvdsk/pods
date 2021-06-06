use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Date {
    Publication(DateTime<Utc>),
    Added(DateTime<Utc>),
}

impl Date {
    pub fn from_item(item: &rss::Item) -> Date {
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
    pub fn inner(&self) -> &DateTime<Utc> {
        match self {
            Self::Publication(d) => d,
            Self::Added(d) => d,
        }
    }
    pub fn age_string(&self) -> String {
        let published: DateTime<Local> = self.inner().clone().into();
        let now = Local::now();
        let since = now.signed_duration_since(published);
        if since.num_days() > 60 {
            return format!("{}", published.format("%d:%m:%Y"));
        }
        if since.num_hours() > 48 {
            return format!("{} days ago", since.num_days());
        }

        format!("{} hours ago", since.num_hours())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Duration(pub f32);
impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let seconds: u64 = self.0 as u64;
        match seconds {
            0..=59 => write!(f, "{}s",seconds),
            60..=899 => write!(f, "{}m:{}s",seconds/60, seconds%60),
            900..=3599 => write!(f, "{}m",seconds/60),
            _ => write!(f, "{}h:{}m", seconds/60/60, seconds/60)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Episode {
    pub title: String,
    /// the duration of the episode in seconds
    pub duration: Duration,
    pub progress: Progress,
    pub date: Date,
}

impl From<&EpisodeExt> for Episode {
    fn from(episode: &EpisodeExt) -> Self {
        Self {
            title: episode.title.to_owned(),
            duration: episode.duration,
            progress: Progress::None,
            date: episode.date,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EpisodeExt {
    pub stream_url: String,
    /// the duration of the episode in seconds
    pub duration: Duration,
    pub title: String,
    pub podcast: String,
    pub date: Date,
    pub description: String,
    // some extra fields
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Podcast {
    pub title: String,
    pub url: String,
}

impl Podcast {
    pub fn from_url(channel: &rss::Channel, url: String) -> Self {
        Self {
            title: channel.title().to_owned(),
            url,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum Progress {
    None,
    Completed,
    Listening(f32),
}

impl Into<f32> for Progress {
    fn into(self) -> f32 {
        match self {
            Progress::None => 0f32,
            Progress::Completed => 0f32,
            Progress::Listening(p) => p,
        }
    }
}
