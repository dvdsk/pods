use super::types::{Episode, Podcast};
use super::error::Error;

// TODO FIXME rewrite using From trait, EpisodeKey should use From PodcastKey

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

impl From<&str> for PodcastKey {
    fn from(podcast: &str) -> Self {
        let hash = hash_str(podcast);
        Self(hash)
    }
}

impl From<&[u8]> for PodcastKey {
    fn from(bytes: &[u8]) -> Self {
        let slice = bytes;
        let mut key = [0u8;8];
        key[0..8].copy_from_slice(slice);
        let hash = u64::from_be_bytes(key);
        Self(hash)
    }
}

impl From<sled::IVec> for PodcastKey {
    fn from(vec: sled::IVec) -> Self {
        PodcastKey::from(vec.as_ref())
    }
}

impl PodcastKey {
    fn podcast_end(&self) -> Self {
        self.increment()
    }
    fn increment(&self) -> Self {
        let hash = self.0;
        hash += 1;
        Self(hash)
    }
}

impl Into<sled::IVec> for PodcastKey {
    fn into(self) -> sled::IVec {
        sled::IVec::from(&self.0.to_be_bytes())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EpisodeKey([u8; 16]);
impl EpisodeKey {

    pub fn from_title(podcast_id: impl Into<PodcastKey>, episode: impl AsRef<str>) -> Self {
        let mut key = [0u8;16];
        let id = podcast_id.into().0.to_be_bytes();
        key[0..8].copy_from_slice(&id);
        let id = hash_str(episode).to_be_bytes();
        key[8..16].copy_from_slice(&id);
        EpisodeKey(key)
    }
    fn podcast_start(podcast_id: impl Into<PodcastKey>) -> Self {
        let mut key = [0u8;16];
        let id = podcast_id.into().0;
        key[0..8].copy_from_slice(&id.to_be_bytes());
        EpisodeKey(key)
    }
    fn podcast_end(podcast_id: impl Into<PodcastKey>) -> Self {
        let mut key = [0u8;16];
        let id = podcast_id.into().0 +1;
        key[0..8].copy_from_slice(&id.to_be_bytes());
        EpisodeKey(key)
    }
}

impl AsRef<[u8]> for EpisodeKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct PodcastDb {
    basic: sled::Tree,
    extended: sled::Tree,
}

impl PodcastDb {
    pub fn open(db: &sled::Db) -> sled::Result<Self> {
        let basic = db.open_tree("podcasts_b_0.1")?;
        let extended = db.open_tree("podcasts_e_0.1")?;
        Ok(Self{
            basic,
            extended,
        })
    }
    pub fn get_podcasts(&self) -> Result<Vec<Podcast>, Error> {
        let mut list = Vec::new();
        let mut key = PodcastKey(0);
        while let Some(kv) = self.basic.get_gt(key.0.to_be_bytes())? {
            let (key_bytes, value) = kv;
            let podcast = bincode::deserialize(&value).unwrap();
            list.push(podcast);

            key = PodcastKey::from(key_bytes);
            key.increment(); // make sure we get another podcast next call
        }
        Ok(list)
    }

    pub fn get_podcast(&self, podcast_id: u64) -> Result<Podcast, Error> {
        let bytes = self.basic.get(podcast_id.to_be_bytes())?.expect("podcast not in database"); 
        let podcast = bincode::deserialize(&bytes).unwrap();
        Ok(podcast)
    }

    pub fn get_episodes(&self, key: impl Into<PodcastKey>) -> Result<Vec<Episode>, Error> {
        let podcast_key = key.into();
        let start = EpisodeKey::podcast_start(podcast_key);
        let end = EpisodeKey::podcast_end(podcast_key);
        let mut list = Vec::new();
        for value in self.basic.range(start..end).values() {
            let episode = bincode::deserialize(&value?).unwrap();
            list.push(episode);
        }
        Ok(list)
    }

    fn update_basic(mut new: Episode, old: Option<&[u8]>) -> Option<impl Into<sled::IVec>>{
        if let Some(existing) = old {
            let existing: Episode = bincode::deserialize(&existing).unwrap();
            new.progress = existing.progress;
        }
        let bytes = bincode::serialize(&new).unwrap();
        Some(bytes)
    }

    pub fn update_episodes(&self, podcast: impl Into<PodcastKey>, new_list: Vec<Episode>) {
        for new in new_list {
            let key = EpisodeKey::from_title(podcast, new.title);
            self.basic.fetch_and_update(key, move |old| Self::update_basic(new, old));
            // self.extended.fetch_and_update(key)
        }
    }
}
