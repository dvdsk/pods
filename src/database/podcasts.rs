use super::error::Error;
use super::types::{Date, Episode, EpisodeExt, Podcast, Progress};

// TODO FIXME rewrite using From trait, EpisodeKey should use From PodcastKey

fn hash_str(s: impl AsRef<str>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.as_ref().hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone, Copy)]
pub struct PodcastKey([u8; 8]);

impl From<&Podcast> for PodcastKey {
    fn from(podcast: &Podcast) -> Self {
        let hash = hash_str(podcast.title.as_str());
        Self(hash.to_be_bytes())
    }
}

impl From<&str> for PodcastKey {
    fn from(podcast: &str) -> Self {
        let hash = hash_str(podcast);
        Self(hash.to_be_bytes())
    }
}

impl From<String> for PodcastKey {
    fn from(podcast: String) -> Self {
        let hash = hash_str(podcast.as_str());
        Self(hash.to_be_bytes())
    }
}

impl From<&[u8]> for PodcastKey {
    fn from(slice: &[u8]) -> Self {
        let mut id = [0u8; 8];
        id[0..8].copy_from_slice(slice);
        Self(id)
    }
}

impl From<sled::IVec> for PodcastKey {
    fn from(vec: sled::IVec) -> Self {
        PodcastKey::from(vec.as_ref())
    }
}

impl PodcastKey {
    // fn podcast_end(&self) -> Self {
    //     self.increment()
    // }
    fn increment(&self) -> Self {
        let mut hash = u64::from_be_bytes(self.0);
        hash += 1;
        Self(hash.to_be_bytes())
    }
}

impl Into<sled::IVec> for PodcastKey {
    fn into(self) -> sled::IVec {
        sled::IVec::from(&self.0)
    }
}

impl AsRef<[u8]> for PodcastKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EpisodeKey([u8; 16]);
impl EpisodeKey {
    pub fn from_title(podcast_id: impl Into<PodcastKey>, episode: impl AsRef<str>) -> Self {
        let mut key = [0u8; 16];
        let id = podcast_id.into().0;
        key[0..8].copy_from_slice(&id);
        let id = hash_str(episode).to_be_bytes();
        key[8..16].copy_from_slice(&id);
        EpisodeKey(key)
    }
    fn podcast_start(podcast_id: impl Into<PodcastKey>) -> Self {
        let mut key = [0u8; 16];
        let id = podcast_id.into().0;
        key[0..8].copy_from_slice(&id);
        EpisodeKey(key)
    }
    fn podcast_end(podcast_id: impl Into<PodcastKey>) -> Self {
        let mut key = [0u8; 16];
        let id = podcast_id.into().increment().0;
        key[0..8].copy_from_slice(&id);
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
        Ok(Self { basic, extended })
    }
    fn next_podcast(&self, id: PodcastKey) -> Result<Option<(PodcastKey, Podcast)>, Error> {
        let res = self.basic.get_gt(id)?;
        if res.is_none() {
            return Ok(None);
        }

        let (key_bytes, value) = res.unwrap();
        if key_bytes.len() != 8 {
            return Ok(None); //no more podcast keys in db
        }

        let podcast = bincode::deserialize(&value).unwrap();
        let id = PodcastKey::from(key_bytes);
        let id = id.increment(); // make sure we get another podcast next call
        Ok(Some((id, podcast)))
    }

    pub fn get_podcasts(&self) -> Result<Vec<Podcast>, Error> {
        let mut list = Vec::new();
        let mut id = PodcastKey([0u8; 8]);
        while let Some((next_id, podcast)) = self.next_podcast(id)? {
            list.push(podcast);
            id = next_id;
        }
        Ok(list)
    }

    pub fn get_podcast(&self, podcast_id: impl Into<PodcastKey>) -> Result<Podcast, Error> {
        let bytes = self
            .basic
            .get(podcast_id.into())?
            .expect("podcast not in database");
        let podcast = bincode::deserialize(&bytes).unwrap();
        Ok(podcast)
    }

    /// add a podcast, if it was already there return an error
    pub fn add_podcast(&self, podcast: &Podcast) -> Result<(), Error> {
        let podcast_id = PodcastKey::from(podcast);
        let bytes = bincode::serialize(&podcast).unwrap();
        let existing = self.basic.insert(podcast_id, bytes)?; 
        if existing.is_some() {
            Err(Error::PodcastAlreadyAdded)
        } else {
            Ok(())
        }
    }

    pub fn get_episodes(&self, podcast_id: impl Into<PodcastKey>) -> Result<Vec<Episode>, Error> {
        let podcast_key = podcast_id.into();
        let start = EpisodeKey::podcast_start(podcast_key);
        let end = EpisodeKey::podcast_end(podcast_key);
        let mut list: Vec<Episode> = Vec::new();
        for value in self.basic.range(start..end).values() {
            let episode = bincode::deserialize(&value?).unwrap();
            list.push(episode);
        }
        Ok(list)
    }

    pub fn get_episode_ext(&self, episode_id: impl Into<EpisodeKey>) -> Result<EpisodeExt, Error> {
        let bytes = self
            .extended
            .get(episode_id.into())?
            .ok_or(Error::NotInDatabase)?;
        let episode = bincode::deserialize(&bytes).unwrap();
        Ok(episode)
    }

    fn update_progress(progress: &Progress, old: Option<&[u8]>) -> impl Into<sled::IVec> {
        let old = old.expect("item should be in database to update progress");
        let mut episode: Episode = bincode::deserialize(&old).unwrap();
        episode.progress = *progress;
        bincode::serialize(&episode).unwrap()
    }

    pub async fn update_episode_progress(&self, episode_id: EpisodeKey, progress: Progress) {
        self.basic
            .fetch_and_update(episode_id, |old| {
                Some(Self::update_progress(&progress, old))
            })
            .unwrap()
            .unwrap();
        self.basic.flush_async().await.unwrap();
    }

    fn update_basic(new: &EpisodeExt, old: Option<&[u8]>) -> impl Into<sled::IVec> {
        let mut new = Episode::from(new);
        if let Some(existing) = old {
            let existing: Episode = bincode::deserialize(&existing).unwrap();
            new.progress = existing.progress;
            if let Date::Added(_) = new.date {
                new.date = existing.date
            }
        }
        bincode::serialize(&new).unwrap()
    }

    fn update_extended(new: &EpisodeExt, _: Option<&[u8]>) -> impl Into<sled::IVec> {
        bincode::serialize(new).unwrap()
    }

    pub fn update_episodes(
        &self,
        podcast_id: impl Into<PodcastKey>,
        new_list: Vec<EpisodeExt>,
    ) -> Result<(), Error> {
        let podcast_id: PodcastKey = podcast_id.into();
        for new in new_list {
            let key = EpisodeKey::from_title(podcast_id, &new.title);
            self.basic
                .fetch_and_update(key, |old| Some(Self::update_basic(&new, old)))?;
            self.extended
                .fetch_and_update(key, |old| Some(Self::update_extended(&new, old)))?;
        }
        Ok(())
    }

    pub async fn update_podcasts(&self) -> Result<(), Error> {
        use crate::feed::add_podcast;

        let mut id = PodcastKey([0u8; 8]);
        while let Some((next_id, podcast)) = self.next_podcast(id)? {
            add_podcast(self.clone(), podcast.url.clone())
                .await
                .unwrap();
            id = next_id;
        }
        Ok(())
    }
}
