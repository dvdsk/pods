use eyre::{Result, WrapErr};
use sled;
use std::path::PathBuf;

pub mod podcasts;
pub use podcasts::Podcasts;
pub mod episodes;
pub use episodes::Episodes;

/* the database contains: 
 *     a table podcasts: with,
 *         key "podcasts" containing a list of podcasts title, url and id. Then a for each 
 *         podcast in that list there is an entry under its id which contains a list of episodes
 *     a table episodes: with,
 *         key episode_id a struct with episode stream url and other episode specific info 
 *         episode_id = hash(podcast_id, episode_name) 
*/

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
