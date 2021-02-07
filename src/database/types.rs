use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Episode {
    pub title: String,
    /// the duration of the episode in seconds
    pub duration: f32,
    pub progress: Progress,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EpisodeExt {
    pub stream_url: String,
    /// the duration of the episode in seconds
    pub duration: f32,
    pub title: String,
    // pub podcast: String
    // some extra fields
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Podcast {
    pub title: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
