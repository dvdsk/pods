use std::time::Duration;
use tokio::sync::mpsc;

use data::Data;
use tokio::time::error::Elapsed;
use tokio::time::timeout;
use traits::{
    DataRStore, DataStore, DataSub, DataUpdate, Episode, Podcast, PodcastId, Registration,
    SearchResult,
};

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

pub struct TestSub {
    rx: mpsc::Receiver<DataUpdate>,
    sub: Box<dyn DataSub>,
    reg: Registration,
}

async fn testsubs(reader: &mut dyn DataRStore) -> Vec<TestSub> {
    let mut subs: Vec<_> = (0..1)
        .into_iter()
        .map(|_| {
            let (tx, rx) = mpsc::channel(10);
            let reg = reader.register(Box::new(tx), "test_sub");
            let sub = reader.sub_podcasts(reg);
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

#[tokio::test]
async fn recieves_current_state() {
    let (mut data, _task) = Data::new();
    let mut writer = data.writer();
    writer.add_podcast(test_podcast(1));

    let mut subs = testsubs(data.reader().as_mut()).await;
    assert!(
        testsubs_got_podcasts(&mut subs, |podcasts| podcasts[0] == test_podcast(1))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn are_updated() {
    let (mut data, _task) = Data::new();
    let mut subs = testsubs(data.reader().as_mut()).await;

    let mut writer = data.writer();
    writer.add_podcast(test_podcast(1));
    assert!(
        testsubs_got_podcasts(&mut subs, |podcasts| podcasts[0] == test_podcast(1))
            .await
            .unwrap()
    );
    writer.add_podcast(test_podcast(2));
    assert!(
        testsubs_got_podcasts(&mut subs, |podcasts| podcasts[1] == test_podcast(2))
            .await
            .unwrap()
    );
}

struct FakeSub;
impl DataSub for FakeSub {}

#[tokio::test]
async fn dropped_are_not_updated() {
    let (mut data, _task) = Data::new();
    let mut subs = testsubs(data.reader().as_mut()).await;

    let mut writer = data.writer();
    writer.add_podcast(test_podcast(1));
    assert!(
        testsubs_got_podcasts(&mut subs, |podcasts| podcasts[0] == test_podcast(1))
            .await
            .unwrap()
    );

    for sub in &mut subs {
        sub.sub = Box::new(FakeSub);
        // old/real sub gets dropped now
        // should no longer be subscribed past here
    }
    writer.add_podcast(test_podcast(2));
    assert!(
        testsubs_got_podcasts(&mut subs, |podcasts| podcasts.len() == 1)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn recieve_episodes() {
    let (mut data, _task) = Data::new();
    let mut subs = testsubs(data.reader().as_mut()).await;

    let mut writer = data.writer();
    writer.add_podcast(test_podcast(1));
    assert!(
        testsubs_got_podcasts(&mut subs, |podcasts| podcasts[0] == test_podcast(1))
            .await
            .unwrap()
    );

    for sub in &mut subs {
        () =false 
        // this drops its sub directly, why does it still work?!?
        data.reader().sub_episodes(sub.reg, 1);
    }
    writer.add_episodes(1, test_episodes());
    assert!(
        testsubs_got_episodes(&mut subs, |id, list| id == 1 && list == test_episodes())
            .await
            .unwrap()
    );
}
