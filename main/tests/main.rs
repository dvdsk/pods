use std::sync::{atomic::AtomicUsize, atomic::Ordering};
use std::time::Duration;

use main::testing::{logging, simulate_user};
use simulate_user::{Steps, ViewableData};

use traits::{DataUpdate, DataUpdateVariant, SearchResult, UserIntent};

#[test]
fn podcasts_are_added() {
    logging::set_error_hook();
    logging::install_tracing();

    let podcast = SearchResult {
        title: "99%Invisible".into(),
        url: "https://feeds.simplecast.com/BqbsxVfO".to_string(),
    };
    let mut podcast_id = None;
    Steps::start_w_timeout(Duration::from_secs(5))
        .then_view(ViewableData::PodcastList)
        .after_data_and(DataUpdateVariant::Podcasts, |update| {
            let DataUpdate::Podcasts { podcasts } = update else { panic!() };
            assert_eq!(podcasts, &vec![]);
            true
        })
        .then_do(UserIntent::AddPodcast(podcast))
        .after_data_and(DataUpdateVariant::Podcasts, |update| {
            let DataUpdate::Podcasts { podcasts } = update else { panic!() };
            podcast_id = podcasts.first().map(|p| p.id);
            true
        })
        .then_stop()
        .run()
        .unwrap();

    assert!(podcast_id.is_some());
}

#[test]
fn adding_episodes_works() {
    logging::set_error_hook();
    logging::install_tracing();

    let podcast = SearchResult {
        title: "99%Invisible".into(),
        url: "https://feeds.simplecast.com/BqbsxVfO".to_string(),
    };
    let podcast_id = AtomicUsize::new(0);
    let mut episodes = None;
    Steps::start_w_timeout(Duration::from_secs(5))
        .then_view(ViewableData::PodcastList)
        .after_data(DataUpdateVariant::Podcasts)
        .then_do(UserIntent::AddPodcast(podcast))
        .after_data_and(DataUpdateVariant::Podcasts, |update| {
            let DataUpdate::Podcasts { podcasts } = update else { panic!() };
            podcast_id.store(podcasts.first().unwrap().id, Ordering::Relaxed);
            true
        })
        .then_view_with(|| ViewableData::Podcast {
            podcast_id: podcast_id.load(Ordering::Relaxed),
        })
        // todo check if we are not just getting the empty list 
        // because thats the first update schedualled
        .after_data_and(
            DataUpdateVariant::Episodes {
                podcast_id: podcast_id.load(Ordering::Relaxed),
            },
            |update| {
                let DataUpdate::Episodes { list, .. } = update else { panic!() };
                episodes = Some(list.clone());
                !list.is_empty()
            },
        )
        .then_stop()
        .run()
        .unwrap();

    assert!(episodes.is_some());
    assert_ne!(episodes.unwrap(), vec![]);
}
