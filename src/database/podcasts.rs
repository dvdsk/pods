use std::collections::HashSet;
use std::str::FromStr;
use eyre::WrapErr;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct PodcastInfo {
    title: String,
    url: String,
    local_id: u64,
}

// TODO exclude listend from partialeq?
// the contains check is invalid with derived
// partialeq
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct EpisodeInfo {
    pub title: String,
    pub listend: bool,
}

pub type EpisodeList = Vec<EpisodeInfo>;
pub type PodcastList = Vec<PodcastInfo>;

pub struct Podcasts {
    tree: sled::Tree,
    db: sled::Db,
}

type LocalId = u64;
impl Podcasts {
    pub fn open(db: &sled::Db) -> sled::Result<Self> {
        let tree = db.open_tree("podcasts")?;
        Ok(Self{
            tree,
            db: db.clone(),
        })
    }
    pub fn add_to_episodelist(&mut self, id: LocalId, list: EpisodeList) 
    -> eyre::Result<()> {

        self.tree.update_and_fetch(id.to_be_bytes(), move |old| {
            if let Some(old) = old {
                let mut episodes: EpisodeList = bincode::deserialize(&old).unwrap();
                let new_episodes: EpisodeList = list.iter().filter(|e| episodes.contains(e)).cloned().collect();
                episodes.extend(new_episodes);
                Some(bincode::serialize(&episodes).unwrap())
            } else {
                let bytes = bincode::serialize(&list).unwrap();
                Some(bytes)
            }
        }).wrap_err("could not update subscribed podcasts in database")?;
        Ok(())
    }
    pub fn add_to_podcastlist(&mut self, title: &str, url: &str)
        -> eyre::Result<LocalId> {
        
        let local_id = self.db.generate_id()?;
        self.tree.update_and_fetch("podcasts", move |old| {
            if let Some(list) = old {
                let mut list: PodcastList = bincode::deserialize(&list).unwrap();
                list.push(PodcastInfo {
                    title: title.to_owned(),
                    url: url.to_owned(),
                    local_id: local_id.to_owned(),
                });
                Some(bincode::serialize(&list).unwrap())
            } else {
                let list: PodcastList = vec!( PodcastInfo {
                    title: title.to_owned(),
                    url: url.to_owned(),
                    local_id: local_id.to_owned(),
                });
                let bytes = bincode::serialize(&list).unwrap();
                Some(bytes)
            }
        }).wrap_err("could not update subscribed podcasts in database")?;
            
        Ok(local_id)
    }
}
