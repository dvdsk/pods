use std::convert::TryFrom;

use serde::{Serialize, Deserialize};
use eyre::eyre;

#[derive(Serialize, Deserialize, Debug)]
pub struct Episode {
    pub stream_url: String,
    // description:
    // author:
    // pub_date:
}

#[derive(Clone, Debug)]
pub struct Episodes {
    tree: sled::Tree,
}

impl TryFrom<&rss::Item> for Episode {
    type Error = eyre::Report;

    fn try_from(item: &rss::Item) -> Result<Self, Self::Error> {
        let stream_url = item.link()
            .ok_or(eyre!("no link for feed item: {:?}", item))?
            .to_owned();
        Ok(Self {
            stream_url,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Key([u8; 8]);
impl From<(&str,&str)> for Key {
    fn from((podcast, episode): (&str, &str)) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        podcast.hash(&mut hasher);
        episode.hash(&mut hasher);
        let key = hasher.finish().to_be_bytes();
        Key(key)
    }
}
impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Episodes {
    pub fn open(db: &sled::Db) -> sled::Result<Self> {
        Ok(Self {
            tree: db.open_tree("episodes")?,
        })
    }
    pub fn add_feed(&mut self, info: rss::Channel) {
        log::info!("adding feed: {}", info.title());
        let podcast_title = &info.title();
        for item in info.items() {
            match Episode::try_from(item) {
                Err(e) => {dbg!(e);()},
                Ok(ep) => {
                    let title = item.title().unwrap();
                    let key = Key::from((*podcast_title, title));
                    self.add(key, ep).unwrap();
                }
            }
        }
    }

    pub fn get(&self, key: Key) -> sled::Result<Episode> {
        log::debug!("key: {:?}", key);
        let bytes = self.tree.get(key)?
            .expect("item should be in database already");
        Ok(bincode::deserialize(&bytes).unwrap())
    }

    pub fn add(&mut self, key: Key, item: Episode) -> sled::Result<()> {
        let bytes = bincode::serialize(&item).unwrap();
        self.tree.insert(key, bytes)?;
            // .expect_none("There should not be an episode with this name already in the db");
        Ok(())
    }

    pub fn remove(&mut self, key: Key) -> sled::Result<()> {
        self.tree.remove(key)?;
        Ok(())
    }
}


