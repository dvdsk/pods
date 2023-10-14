use media::Media;
use traits::Media as _;

#[tokio::test]
async fn test() {
    let (mut media, err_h) = Media::new();
    let episode_id = 0;
    media.download(episode_id);
}
