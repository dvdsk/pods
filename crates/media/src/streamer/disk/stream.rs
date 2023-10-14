use super::StreamErr;

pub(crate) struct New {
    pub url: url::Url,
}
pub(super) async fn process(new: New) -> Result<(), StreamErr> {
    let New { url } = new;
    let mut client = reqwest::Client::new();
    let mut response = client.get(url).send().await.unwrap();
    let mut downloaded = 0;
    while let Some(chunk) = response.chunk().await.unwrap() {
        // todo stream?


    }

    Ok(())
}
