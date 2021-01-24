use iced_futures::futures;
use std::path::PathBuf;
use super::Download;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Progress {
    Started,
    Advanced(f32),
    Finished,
    Error(Error),
}

type Result<T> = std::result::Result<T, Error>;
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Problem connecting to download")]
    Download(#[from] Arc<reqwest::Error>),
    #[error("Could not store download")]
    Io(#[from] Arc<std::io::Error>),
}

impl<H, I> iced_futures::subscription::Recipe<H, I> for Download 
where 
    H: std::hash::Hasher,
{
    type Output = Progress;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;
        std::any::TypeId::of::<Self>().hash(state);
        self.url.hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: futures::stream::BoxStream<'static, I>,
    ) -> futures::stream::BoxStream<'static, Self::Output> {
        Box::pin(
            futures::stream::unfold(
                State::Start(self.url, self.path), |state| async move {
                    stream_state_machine(state).await
                }
            )
        )
    }
}

type StateResult = Option<(Progress, State)>;
async fn stream_state_machine(current: State) -> StateResult {
    match current {
        State::Start(url, path) => 
            start(url,path).await.unwrap_or_else(
                |e| Some((Progress::Error(e), State::Finished))),
        State::Downloading(data) =>
            downloading(data).await.unwrap_or_else(
                |e| Some((Progress::Error(e), State::Finished))),
        State::Finished => None
    }
}

async fn start(url: String, path: PathBuf) -> Result<StateResult> {
    log::debug!("downloading url: {}", &url);
    let res = reqwest::get(&url).await.map_err(Arc::from)?;
    let total = res.content_length();
    let file = File::create(path).map_err(Arc::from)?;
    let state = DownloadData {
        res, file, total, downloaded: 0,
    };
    Ok(Some((Progress::Started, State::Downloading(state))))
}

async fn downloading(mut data: DownloadData) -> Result<StateResult> {
    let DownloadData {res, file, total, downloaded} = &mut data;
    match res.chunk().await.map_err(Arc::from)? {
        // (e) => Some((Progress::Error(e.to_string()), State::Finished)),
        None => Ok(Some((Progress::Finished, State::Finished))),
        Some(chunk) => {
            *downloaded += chunk.len() as u64;
            file.write_all(&chunk).map_err(Arc::from)?;

            let percentage = total
                .map(|t| 100.0 * *downloaded as f32/ t as f32)
                .unwrap_or(0.0);
            let progress = Progress::Advanced(percentage);
            Ok(Some((progress, State::Downloading(data))))
        }
    }
}

#[derive(Debug)]
pub struct DownloadData {
    res: reqwest::Response,
    file: File,
    total: Option<u64>,
    downloaded: u64,
}

#[derive(Debug)]
pub enum State {
    Start(String, PathBuf),
    Downloading(DownloadData),
    Finished,
}
