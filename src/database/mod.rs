use eyre::{Result, WrapErr};
use std::path::PathBuf;

mod error;
mod podcasts;
mod types;

pub use error::Error;
pub use podcasts::{EpisodeKey, PodcastDb, PodcastKey};
pub use types::{Date, Episode, EpisodeExt, Podcast, Progress};

pub fn open() -> Result<sled::Db> {
    let path = PathBuf::from("database");
    let config = sled::Config::default()
        .path(&path)
        .cache_capacity(10_000_000) //10mb
        .flush_every_ms(Some(1000));
    let db = config
        .open()
        .wrap_err_with(|| format!("Could not open database on {:?}", path))?;
    Ok(db)
}
