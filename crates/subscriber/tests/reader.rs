use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use subscriber::{PublishTask, Publisher};
use tokio::sync::mpsc;
use tokio::time::timeout;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Update(usize);
#[derive(Debug, Clone, PartialEq, Eq, std::hash::Hash)]
struct Key(usize);

impl From<&Update> for Key {
    fn from(value: &Update) -> Self {
        Self(value.0 / 10)
    }
}
type Data = Arc<Mutex<HashMap<Key, usize>>>;

fn test_publisher() -> (Publisher<Update, Key>, PublishTask, Data) {
    let data: Data = Arc::new(Mutex::new(HashMap::new()));
    let update_source = {
        let data = data.clone();
        move |key: &Key| {
            let item = data.lock().unwrap().get(key).unwrap().clone();
            Update(item)
        }
    };
    let (publisher, task) = Publisher::new(update_source);
    (publisher, task, data)
}

#[tokio::test]
async fn dropped_subs_are_not_update() {
    let (publisher, _task, data) = test_publisher();
    data.lock().unwrap().insert(Key(1), 10);

    let (tx, mut rx) = mpsc::channel(4);
    let reg = publisher.register(tx, "test sub");
    let sub = publisher.subscribe(reg, Key(1));
    assert_eq!(rx.recv().await.unwrap(), Update(10));
    std::mem::drop(sub);

    publisher.publish(&Update(13));
    let res = timeout(Duration::from_secs(1), rx.recv()).await;
    assert!(
        res.is_err(),
        "Should time out as publisher no longer sends data"
    );
}

#[tokio::test]
async fn published_data_arrives_everywhere() {
    let (publisher, _task, data) = test_publisher();
    data.lock().unwrap().insert(Key(1), 10);

    let mut subs = Vec::new();
    for _ in 0..10 {
        let (tx, mut rx) = mpsc::channel(10);
        let reg = publisher.register(tx, "test sub");
        let sub = publisher.subscribe(reg, Key(1));
        rx.recv().await.unwrap();
        subs.push((rx, sub));
    }

    data.lock().unwrap().insert(Key(1), 13);
    publisher.publish(&Update(13));
    for (mut rx, _sub) in subs {
        assert_eq!(rx.recv().await.unwrap(), Update(13))
    }
}

#[tokio::test]
async fn on_subscribe_get_latest_data() {
    let (publisher, _task, data) = test_publisher();
    data.lock().unwrap().insert(Key(20), 10);

    let (tx, mut rx) = mpsc::channel(10);
    let reg = publisher.register(tx, "test sub");
    let _sub = publisher.subscribe(reg, Key(20));
    assert_eq!(rx.recv().await.unwrap(), Update(10))
}
