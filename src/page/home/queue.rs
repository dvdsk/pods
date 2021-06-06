use crate::database::EpisodeKey;

#[derive(Default, Debug)]
pub struct Queue(Vec<EpisodeKey>);

impl Queue {
    pub fn add(&mut self, _key: EpisodeKey) {
        todo!();
    }
}
