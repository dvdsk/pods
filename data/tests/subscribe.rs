use std::time::Duration;
use tokio::sync::mpsc;

use data::Data;
use tokio::time::timeout;
use traits::{
    DataRStore, DataStore, DataSub, DataUpdate, Podcast, PodcastId, Registration, SearchResult,
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

pub struct TestSub {
    rx: mpsc::Receiver<DataUpdate>,
    sub: Box<dyn DataSub>,
    reg: Registration,
}

fn testsubs(reader: &mut dyn DataRStore) -> Vec<TestSub> {
    (0..10)
        .into_iter()
        .map(|_| {
            let (tx, rx) = mpsc::channel(10);
            let reg = reader.register(Box::new(tx), "test_sub");
            let sub = reader.sub_podcasts(reg);
            TestSub { rx, sub, reg }
        })
        .collect()
}

async fn testsubs_got_update(list: &mut [TestSub], podcast_id: PodcastId) -> bool {
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
        updates.all(|podcast| async move { podcast.first() == Some(&test_podcast(podcast_id)) }),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn subscribers_recieves_current_state() {
    let mut data = Data::new();
    let mut writer = data.writer();
    writer.add_podcast(test_podcast(1));

    let mut subs = testsubs(data.reader().as_mut());
    assert!(testsubs_got_update(&mut subs, 1).await);
}

#[tokio::test]
async fn subscribers_are_updated() {
    let mut data = Data::new();
    let mut subs = testsubs(data.reader().as_mut());

    let mut writer = data.writer();
    writer.add_podcast(test_podcast(1));
    assert!(testsubs_got_update(&mut subs, 1).await);
    writer.add_podcast(test_podcast(2));
    assert!(testsubs_got_update(&mut subs, 2).await);
}

struct FakeSub;
impl DataSub for FakeSub {}

#[tokio::test]
async fn dropped_subscribers_are_not_updated() {
    let mut data = Data::new();
    let mut subs = testsubs(data.reader().as_mut());

    let mut writer = data.writer();
    writer.add_podcast(test_podcast(1));
    assert!(testsubs_got_update(&mut subs, 1).await);

    for sub in &mut subs {
        sub.sub = Box::new(FakeSub);
        // old/real sub gets dropped now
        // should no longer be subscribed past here
    }
    writer.add_podcast(test_podcast(2));
    assert!(!testsubs_got_update(&mut subs, 2).await);
}
