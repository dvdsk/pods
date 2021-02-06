use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Episode {
    pub stream_url: String,
    /// the duration of the episode in seconds
    pub duration: f32,
    pub title: String,
    pub podcast: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EpisodeExt {
    pub stream_url: String,
    /// the duration of the episode in seconds
    pub duration: f32,
    pub title: String,
    pub podcast: String
    // some extra fields
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Podcast {
    pub title: String,
}
