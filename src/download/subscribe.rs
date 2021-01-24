use iced_futures::futures;
use std::path::PathBuf;
use super::Download;
use std::fs::{self, File};
use std::io::Write;
use std::sync::Arc;
use error_level::ErrorLevel;

#[derive(Debug, Clone)]
pub enum Progress {
    Started,
    Advanced(f32),
    Finished,
    Error(Error),
}

type Result<T> = std::result::Result<T, Error>;
#[derive(thiserror::Error, ErrorLevel, Debug, Clone)]
pub enum Error {
    #[report(warn)]
    #[error("Problem connecting to download")]
    Download(#[from] Arc<reqwest::Error>),
    #[report(error)]
    #[error("Could not store download")]
    Io(#[from] Arc<std::io::Error>),
    #[report(warn)]
    #[error("Do not know what file type this is (no extension given)")]
    NoExtension,
}

impl<H, I> iced_futures::subscription::Recipe<H, I> for Download 
where 
    H: std::hash::Hasher,
{
    type Output = Progress;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;
        std::any::TypeId::of::<Self>().hash(state);
        self.url.as_str().hash(state);
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
                |e| Some((Progress::Error(e), State::Errored))),
        State::Downloading(data) =>
            downloading(data).await.unwrap_or_else(
                |e| Some((Progress::Error(e), State::Errored))),
        State::Finished(temp_path) => {
            let mut path = temp_path.clone(); // name.extension.part
            path.set_extension(""); // this removes the .part 
            fs::rename(temp_path, path).unwrap();
            None
        }
        State::Errored => None,
    }
}

async fn start(url: reqwest::Url, path: PathBuf) -> Result<StateResult> {
    log::info!("downloading to file: {}", &path.to_string_lossy());
    let res = reqwest::get(url).await.map_err(Arc::from)?;
    let total = res.content_length();
    let dir = path.parent().unwrap().parent().unwrap();
    fs::create_dir_all(dir).map_err(Arc::from)?;
    let file = File::create(&path).map_err(Arc::from)?;
    let state = DownloadData {
        res, file, total, downloaded: 0, path,
    };
    Ok(Some((Progress::Started, State::Downloading(state))))
}

async fn downloading(data: DownloadData) -> Result<StateResult> {
    let DownloadData {mut res, mut file, total, mut downloaded, path} = data;
    match res.chunk().await.map_err(Arc::from)? {
        None => Ok(Some((Progress::Finished, State::Finished(path)))),
        Some(chunk) => {
            downloaded += chunk.len() as u64;
            file.write_all(&chunk).map_err(Arc::from)?;

            let percentage = total
                .map(|t| 100.0 * downloaded as f32/ t as f32)
                .unwrap_or(0.0);
            let progress = Progress::Advanced(percentage);
            let data = DownloadData {res, file, total, downloaded, path};
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
    path: PathBuf,
}

#[derive(Debug)]
pub enum State {
    Start(reqwest::Url, PathBuf),
    Downloading(DownloadData),
    Finished(PathBuf),
    Errored,
}
