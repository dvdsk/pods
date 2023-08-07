use traits::{DataUpdate, Podcast};

#[dbstruct::dbstruct(db=sled)]
pub struct Store {
    pub podcasts: Vec<Podcast>,
}

impl Store {
    pub(crate) fn podcast_update(&self) -> DataUpdate {
        let list = self
            .podcasts()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        DataUpdate::Podcasts { podcasts: list }
    }
}
