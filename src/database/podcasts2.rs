use super::types::{Episode, Podcast};
use super::error::Error;

fn hash_str(s: impl AsRef<str>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.as_ref().hash(&mut hasher);
    let key = hasher.finish();
    key
}

#[derive(Debug, Clone, Copy)]
pub struct PodcastKey(u64);

impl PodcastKey {
    fn podcast_start(podcast: &str) -> Self {
        let hash = hash_str(podcast);
        Self(hash)
    }
    fn podcast_end(&self) -> Self {
        self.increment()
    }
    fn increment(&self) -> Self {
        let hash = self.0;
        hash += 1;
        Self(hash)
    }
    fn from_slice(vec: impl AsRef<[u8]>) -> Self {
        let slice = vec.as_ref();
        let mut key = [0u8;8];
        key[0..8].copy_from_slice(slice);
        let hash = u64::from_be_bytes(key);
        PodcastKey(hash)
    }
}

impl AsRef<[u8]> for PodcastKey {
    fn as_ref(&self) -> &[u8] {
        &self.0.to_be_bytes()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EpisodeKey([u8; 16]);
impl EpisodeKey {

    fn episode(podcast: impl AsRef<str>, episode: impl AsRef<str>) -> Self {
        let mut key = [0u8;16];
        let hash = hash_str(podcast).to_be_bytes();
        key[0..8].copy_from_slice(&hash);
        let hash = hash_str(episode).to_be_bytes();
        key[8..16].copy_from_slice(&hash);
        EpisodeKey(key)
    }
    fn podcast_start(podcast: impl AsRef<str>) -> Self {
        let mut key = [0u8;16];
        let hash = hash_str(podcast);
        key[0..8].copy_from_slice(&hash.to_be_bytes());
        EpisodeKey(key)
    }
    fn podcast_end(podcast: &str) -> Self {
        let mut key = [0u8;16];
        let hash = hash_str(podcast) +1;
        key[0..8].copy_from_slice(&hash.to_be_bytes());
        EpisodeKey(key)
    }

}

impl AsRef<[u8]> for EpisodeKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

pub struct PodcastDb {
    basic: sled::Tree,
    extended: sled::Tree,
}

impl PodcastDb {
    pub fn get_podcasts(&self) -> Result<Vec<Podcast>, Error> {
        let mut list = Vec::new();
        let mut key = PodcastKey(0);
        while let Some(kv) = self.basic.get_gt(&key)? {
            let (key_bytes, value) = kv;
            let podcast = bincode::deserialize(&value).unwrap();
            list.push(podcast);

            key = PodcastKey::from_slice(key_bytes);
            key.increment(); // make sure we get another podcast next call
        }
        Ok(list)
    }

    pub fn get_episodes(&self, podcast: &str) -> Result<Vec<Episode>, Error> {
        let start = EpisodeKey::podcast_start(podcast);
        let end = EpisodeKey::podcast_end(podcast);
        let mut list = Vec::new();
        for value in self.basic.range(start..end).values() {
            let episode = bincode::deserialize(&value?).unwrap();
            list.push(episode);
        }
        Ok(list)
    }

    fn update_basic(mut new: Episode, old: Option<&[u8]>) -> Option<impl Into<sled::IVec>>{
        if let Some(existing) = old {
            let existing = bincode::deserialize(&existing).unwrap();
            new.progress = existing.progress;
        }
        let bytes = bincode::serialize(&new).unwrap();
        Some(bytes)
    }

    pub fn update_episodes(&self, new_list: Vec<Episode>) {
        for new in new_list {
            let key = EpisodeKey::episode(new.podcast, new.title);
            self.basic.fetch_and_update(key, move |old| Self::update_basic(new, old));
            // self.extended.fetch_and_update(key)
        }
    }
}
