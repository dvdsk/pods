use eyre::WrapErr;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct PodcastInfo {
    pub title: String,
    url: String,
    pub local_id: u64,
}

// TODO exclude listend from partialeq?
// the contains check is invalid with derived
// partialeq
#[derive(Serialize, Deserialize, Clone)]
pub struct EpisodeInfo {
    pub title: String,
    pub listend: bool,
}

impl PartialEq for EpisodeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
    }
}

pub type EpisodeList = Vec<EpisodeInfo>;
pub type PodcastList = Vec<PodcastInfo>;

#[derive(Clone)]
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
    pub fn get_podcastlist(&mut self) -> sled::Result<PodcastList> {
        if let Some(data) = self.tree.get("podcasts")? {
            let list = bincode::deserialize(&data).unwrap();
            Ok(list)
        } else {
            Ok(Vec::new())
        }
    }
    pub fn get_episodelist(&mut self, id: LocalId) -> sled::Result<EpisodeList> {
        let data = self.tree.get(id.to_be_bytes())?.expect("every podcast should have an episode list");
        let list = bincode::deserialize(&data).unwrap();
        Ok(list)
    }
}
