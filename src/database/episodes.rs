use std::convert::TryFrom;
use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use eyre::eyre;

#[derive(Serialize, Deserialize, Debug)]
pub struct Episode {
    pub stream_url: String,
    /// the duration of the episode in seconds
    pub duration: f32,
    pub title: String,
    pub podcast: String,
    // description:
    // author:
    // pub_date:
}

impl Episode {
    fn file_path(&self) -> PathBuf {
        use directories::UserDirs;
        let user_dirs = UserDirs::new()
            .expect("can not download if the user has no home directory");
        let mut dl_dir = user_dirs.download_dir()
            .expect("need a download folder to be able to download")
            .to_owned();
        dl_dir.push(env!("CARGO_BIN_NAME"));
        dl_dir.push(&self.podcast);
        dl_dir.push(&self.title);
        dl_dir.push("mp3.tmp");
        dl_dir
    }
    pub fn temp_file_path(&self) -> PathBuf {
        let mut dl_dir = self.file_path();
        dl_dir.push(".part");
        dl_dir
    }
}

#[derive(Clone, Debug)]
pub struct Episodes {
    tree: sled::Tree,
}

fn url_from_extensions(item: &rss::Item) -> Option<String> {
    let media = item.extensions().get("media")?;
    let content = media.get("content")?;
    let extention = content.first()?;
    if extention.name() != "media:content" { return None }
    extention.attrs().get("url").map(|u| u.clone())
}

fn length_from_extensions(item: &rss::Item) -> Option<f32> {
    let media = item.extensions().get("media")?;
    // dbg!(&media); 
    let content = media.get("content")?;
    let extention = content.first()?;
    if extention.name() != "media:content" { return None }
    // TODO FIXME find out what the key could be here
    // extention.attrs().get("url").map(|u| u.parse().ok()).flatten()
    None
}

impl TryFrom<(&rss::Item, &str)> for Episode {
    type Error = eyre::Report;

    fn try_from((item, podcast_title): (&rss::Item, &str)) -> Result<Self, Self::Error> {
        //try to get the url and duration from the description of the media object
        let stream_url = item.enclosure().map(|encl| encl.url().to_owned());
        let duration = item.enclosure().map(|encl| encl.length().parse().ok()).flatten();

        //try to get the url and duration possible extensions
        let stream_url = stream_url.or(url_from_extensions(item));
        let duration = duration.or(length_from_extensions(item));

        //try to get duration from any included itunes extensions
        let duration = duration.or(item.itunes_ext()
            .map(|ext| ext.duration()
                .map(|d| d.parse().ok()).flatten()
            ).flatten());

        let stream_url = stream_url.ok_or(eyre!("no link for feed item: {:?}", item))?;
        let duration = duration.ok_or(eyre!("no duration known for item: {:?}", item))?;
        let title = item.title().ok_or(eyre!("episode should have a title: {:?}", item))?.to_owned();
        let podcast = podcast_title.to_owned();

        Ok(Self {
            stream_url,
            duration,
            title,
            podcast,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Key([u8; 8]);
impl From<(u64,&str)> for Key {
    fn from((podcast_id, episode): (u64, &str)) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        podcast_id.hash(&mut hasher);
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
    pub fn add_feed(&mut self, id: u64, info: rss::Channel) {
        log::info!("adding feed: {}", info.title());
        let podcast_title = info.title();
        for item in info.items() {
            match Episode::try_from((item, podcast_title)) {
                Err(e) => {dbg!(e);()},
                Ok(ep) => {
                    let title = item.title().expect("episode has not title");
                    let key = Key::from((id, title));
                    self.add(key, ep).unwrap();
                }
            }
        }
    }

    pub fn get(&self, key: Key) -> sled::Result<Episode> {
        log::debug!("getting episode, key: {:?}", key);
        let bytes = self.tree.get(key)?
            .expect("item should be in database already");
        Ok(bincode::deserialize(&bytes).unwrap())
    }

    pub fn add(&mut self, key: Key, item: Episode) -> sled::Result<()> {
        log::debug!("adding episode, key: {:?}", key);
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


