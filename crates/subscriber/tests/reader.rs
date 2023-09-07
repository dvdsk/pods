
use std::time::Duration;
use tokio::sync::mpsc;

use tokio::task::JoinHandle;
use tokio::time::error::Elapsed;
use tokio::time::timeout;
use traits::{
    DataSub, DataUpdate, Episode, EpisodeDetails, EpisodeId, Podcast,
    PodcastId, Registration, SearchResult,
};
use subscriber::{Clients, ClientsMap, Reader};

subscriber::Subs! {
     podcast Clients,
     episodes ClientsMap<PodcastId>
}

#[derive(Debug)]
pub enum Needed {
    PodcastList,
    Episodes(PodcastId),
}

impl subscriber::Needed<Data, Subs> for Needed {
    fn subs(&self, subs: &Subs) -> Vec<Registration> {
        match self {
            Needed::PodcastList => subs.podcast.regs(),
            Needed::Episodes(podcast_id) => subs.episodes.regs(podcast_id),
        }
    }

    fn update(&self, data: &Data) -> DataUpdate {
        match self {
            Needed::PodcastList => data.podcast_update(),
            Needed::Episodes(podcast_id) => data.episodes_update(*podcast_id),
        }
    }
}

#[derive(Debug, Default)]
struct DataInner {
    field: ()
}

#[derive(Debug, Default, Clone)]
struct Data {
    inner: Arc<Mutex<DataInner>>,
}

impl Data {
    fn podcast_update(&self) -> DataUpdate { todo!() }
    fn episodes_update(&self, podcast_id: PodcastId) -> DataUpdate { todo!() }

    fn add_episodes(&self, podcast_id: PodcastId, episode: Vec<Episode>) { todo!()}
    fn add_podcast(&self, podcast: Podcast) { todo!()}

    fn register(&self, client: Box<dyn traits::DataTx>, client_description: &'static str) -> Registration {todo!() }
    fn sub_episodes(&self, reg: Registration, podcast_id: PodcastId) { todo!()}
    fn sub_podcasts(&self, reg: Registration) -> Box<dyn DataSub> { todo!()}
}

use std::sync::{Mutex, Arc};
fn new_data() -> (Data, Reader<Needed, Data, Subs>, JoinHandle<()>) {
    let data = Data::default();
    let subs = Subs::default();
    let (reader, task) = Reader::new(data.clone(), subs);
    (data, reader, task)
}

fn test_podcast(id: PodcastId) -> Podcast {
    Podcast::try_from_searchres(
        SearchResult {
            title: String::from("test podcast"),
            url: String::from("https://www.example.org"),
        },
        id,
    )
    .unwrap()
}

fn test_episodes() -> Vec<Episode> {
    (0..1)
        .into_iter()
        .map(|id| Episode {
            id,
            name: format!("Test Episode {id}"),
        })
        .collect()
}

fn test_episode_details(id: EpisodeId) -> EpisodeDetails {
    use chrono::{NaiveDate, Utc};
    let date = NaiveDate::from_ymd_opt(2014, 7, 8)
        .unwrap()
        .and_hms_opt(9, 10, 11)
        .unwrap()
        .and_local_timezone(Utc)
        .unwrap();
    EpisodeDetails {
        episode_id: id,
        date: traits::Date::Publication(date),
        duration: Duration::from_secs(10),
        description: String::from("description"),
    }
}

pub struct TestSub {
    rx: mpsc::Receiver<DataUpdate>,
    sub: Box<dyn DataSub>,
    reg: Registration,
}

async fn testsubs(data: &mut Data) -> Vec<TestSub> {
    let mut subs: Vec<_> = (0..1)
        .into_iter()
        .map(|_| {
            let (tx, rx) = mpsc::channel(10);
            let reg = data.register(Box::new(tx), "test_sub");
            let sub = data.sub_podcasts(reg);
            TestSub { rx, sub, reg }
        })
        .collect();

    // get the initial update out
    testsubs_got_podcasts(&mut subs, |podcasts| podcasts.is_empty())
        .await
        .unwrap();
    subs
}

async fn testsubs_got_podcasts(
    list: &mut [TestSub],
    predicate: impl Fn(Vec<Podcast>) -> bool,
) -> Result<bool, Elapsed> {
    use futures::stream::FuturesUnordered;
    use futures::StreamExt;
    let updates: FuturesUnordered<_> = list
        .iter_mut()
        .map(|s| async {
            let DataUpdate::Podcasts{ podcasts } = s.rx.recv().await.unwrap() else {
                    panic!("wrong update");
                };
            podcasts
        })
        .collect();
    timeout(
        Duration::from_secs(1),
        updates.all(|podcast| async { predicate(podcast) }),
    )
    .await
}

async fn testsubs_got_episodes(
    list: &mut [TestSub],
    predicate: impl Fn(PodcastId, Vec<Episode>) -> bool + Copy,
) -> Result<bool, Elapsed> {
    use futures::stream::FuturesUnordered;
    use futures::StreamExt;
    let updates: FuturesUnordered<_> = list
        .iter_mut()
        .map(|s| async {
            let DataUpdate::Episodes{ podcast_id, list } = s.rx.recv().await.unwrap() else {
                    panic!("wrong update");
                };
            (podcast_id, list)
        })
        .collect();
    timeout(
        Duration::from_secs(1),
        updates.all(|(id, list)| async move { predicate(id, list) }),
    )
    .await
}

async fn testsubs_got_episodes_details(
    list: &mut [TestSub],
    predicate: impl Fn(EpisodeDetails) -> bool + Copy,
) -> Result<bool, Elapsed> {
    use futures::stream::FuturesUnordered;
    use futures::StreamExt;
    let updates: FuturesUnordered<_> = list
        .iter_mut()
        .map(|s| async {
            let DataUpdate::EpisodeDetails{ details } = s.rx.recv().await.unwrap() else {
                    panic!("wrong update");
                };
            details
        })
        .collect();
    timeout(
        Duration::from_secs(1),
        updates.all(|details| async move { predicate(details) }),
    )
    .await
}


#[tokio::test]
async fn recieves_current_state() {
    let (mut data, reader, _task) = new_data();
    data.add_podcast(test_podcast(1));

    let mut subs = testsubs(&mut data).await;
    assert!(
        testsubs_got_podcasts(&mut subs, |podcasts| podcasts[0] == test_podcast(1))
            .await
            .unwrap()
    );
}

// #[tokio::test]
// async fn are_updated() {
//     let (mut data, reader, _task) = new_data();
//     let mut subs = testsubs(&mut data).await;
//
//     data.add_podcast(test_podcast(1));
//     assert!(
//         testsubs_got_podcasts(&mut subs, |podcasts| podcasts[0] == test_podcast(1))
//             .await
//             .unwrap()
//     );
//     data.add_podcast(test_podcast(2));
//     assert!(
//         testsubs_got_podcasts(&mut subs, |podcasts| podcasts[1] == test_podcast(2))
//             .await
//             .unwrap()
//     );
// }
//
// struct FakeSub;
// impl DataSub for FakeSub {}
//
// #[tokio::test]
// async fn dropped_are_not_updated() {
//     let (mut data, reader, _task) = new_data();
//     let mut subs = testsubs(&mut data).await;
//
//     data.add_podcast(test_podcast(1));
//     assert!(
//         testsubs_got_podcasts(&mut subs, |podcasts| podcasts[0] == test_podcast(1))
//             .await
//             .unwrap()
//     );
//
//     for sub in &mut subs {
//         sub.sub = Box::new(FakeSub);
//         // old/real sub gets dropped now
//         // should no longer be subscribed past here
//     }
//     data.add_podcast(test_podcast(2));
//     assert!(
//         testsubs_got_podcasts(&mut subs, |podcasts| podcasts.len() == 1)
//             .await
//             .is_err()
//     );
// }
//
// #[tokio::test]
// async fn recieve_episodes() {
//     let (mut data, reader, _task) = new_data();
//     let mut subs = testsubs(&mut data).await;
//
//     let mut episode_subs = Vec::new();
//     for test_sub in &mut subs {
//         let sub = data.sub_episodes(test_sub.reg, 1);
//         episode_subs.push(sub)
//     }
//     data.add_episodes(1, test_episodes());
//     assert!(
//         testsubs_got_episodes(&mut subs, |id, list| id == 1 && list == test_episodes())
//             .await
//             .unwrap()
//     );
// }
