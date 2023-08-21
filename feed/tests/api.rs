use feed::Feed;
use traits::Feed as _;
use traits::Podcast;
use traits::Date;

fn podcast() -> Podcast {
    Podcast {
        name: "99%Invisible".into(),
        feed: url::Url::parse("https://feeds.simplecast.com/BqbsxVfO").unwrap(),
        id: 0,
    }
}

#[tokio::test]
async fn index() {
    let feed = Feed::new();
    let episodes = feed.index(&podcast()).await.unwrap();
    assert!(!episodes.is_empty());
    assert!(matches!(episodes.first().unwrap().date, Date::Publication(_)))
}
