use stream_owl::Manager;
use std::io::Read;

const URL: &str = "https://dts.podtrac.com/redirect.mp3/chrt.fm/track/288D49/stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/c660ce6b-ced1-459f-9535-113c670e83c9/audio/128/default.mp3?aid=rss_feed&awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&awEpisodeId=c660ce6b-ced1-459f-9535-113c670e83c9&feed=BqbsxVfO";

fn main() {
    let mut streams = Manager::new();
    let handle = streams.add(URL);
    let reader = handle.try_get_reader().unwrap();

    let mut buf = vec![0u8; 1024];
    reader.read(&mut buf);
    assert!(buf.iter().filter(|i| **i == 0).count() < 100);
}
