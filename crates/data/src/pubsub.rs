use std::sync::Arc;

use traits::{DataUpdate, DataUpdateVariant};

use crate::db;

use subscriber::PublishTask;
pub(crate) type Publisher = subscriber::Publisher<DataUpdate, DataUpdateVariant>;

fn update(key: &DataUpdateVariant, data: &db::Store) -> DataUpdate {
    use DataUpdateVariant as D;
    match key {
        D::Podcasts => data.podcast_update(),
        D::Episodes { podcast_id } => data.episodes_update(*podcast_id),
        D::EpisodeDetails { episode_id } => data.episode_details_update(*episode_id),
        D::Downloads => todo!(),
    }
}

pub fn new(data: Arc<db::Store>) -> (Publisher, PublishTask) {
    let update_source = move |key: &DataUpdateVariant| update(key, &data);
    Publisher::new(update_source)
}
