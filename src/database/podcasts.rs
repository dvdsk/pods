use eyre::WrapErr;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct PodcastInfo {
    title: String,
    url: String,
    local_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct EpisodeInfo {
    title: String,
    listend: bool,
}

type EpisodeList = Vec<EpisodeInfo>;
type PodcastList = Vec<PodcastInfo>;

pub struct Podcasts {
    tree: sled::Tree,
    db: sled::Db,
}

impl Podcasts {
    pub fn open(db: &sled::Db) -> sled::Result<Self> {
        let tree = db.open_tree("podcasts")?;
        Ok(Self{
            tree,
            db: db.clone(),
        })
    }

    pub fn add(&mut self, title: String, url: String)
        -> eyre::Result<()> {
        
        // let local_id = self.db.generate_id()?;
        // self.tree.update_and_fetch("podcasts", move |old| {
        //     if let Some(list) = old {
        //         let mut list: PodcastList = bincode::deserialize(&list).unwrap();
        //         list.push(PodcastInfo {
        //             title,
        //             url,
        //             local_id,
        //         });
        //         Some(bincode::serialize(&list).unwrap())
        //     } else {
        //         let list: PodcastList = vec!( PodcastInfo {
        //             title,
        //             url,
        //             local_id: local_id,
        //         });
        //         let bytes = bincode::serialize(&list).unwrap();
        //         Some(bytes)
        //     }
        // });
            

        // let mut list = if let Some(item) = self.tree
        //     .get(&title)
        //     .wrap_err("database error while adding podcast")? {
        //     bincode::deserialize(&item).unwrap()
        // } else {
        //     Vec::new()
        // };

        // list.push(PodcastInfo {
        //     title,
        //     url,
        //     local_id: self.db.generate_id()?,
        // });

        // self.tree.

        

        Ok(())
    }
}
