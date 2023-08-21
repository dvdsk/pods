use main::testing::{logging, simulate_user};
use simulate_user::{Steps, ViewableData};

use traits::{UserIntent, DataUpdate, DataUpdateVariant, SearchResult};

#[tokio::test]
async fn add_podcast_look_at_episodes() {
    logging::set_error_hook();
    logging::install_tracing();

    let podcast = SearchResult {
        title: "99%Invisible".into(),
        url: "https://feeds.simplecast.com/BqbsxVfO".to_string(),
    };
    let mut podcast_id = None;
    Steps::start()
        .then_do(UserIntent::AddPodcast(podcast))
        .after_data(DataUpdateVariant::Podcast)
        .then_view(ViewableData::PodcastList)
        .after_data_and(DataUpdateVariant::Podcast, |update| {
            let DataUpdate::Podcasts { podcasts } = update else { panic!() };
            podcast_id = podcasts.first().map(|p| p.id);
            true
        })
        .then_stop()
        .run().await;

    assert!(podcast_id.is_some());
}
