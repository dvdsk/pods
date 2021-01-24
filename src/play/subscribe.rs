use iced_futures::futures;
use std::sync::{Arc, Mutex, mpsc};

pub fn play(url: String) -> iced::Subscription<Progress> {
    iced::Subscription::from_recipe(Stream {url})
}

#[derive(Debug, Clone)]
pub enum Progress {
    Started(Arc<Mutex<mpsc::Receiver<bytes::Bytes>>>),
    Advanced(f32),
    Finished,
    StreamError(String),
    ToShortError,
}

pub struct Stream {
    url: String,
}

impl<H, I> iced_futures::subscription::Recipe<H, I> for Stream 
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
                State::Start(self.url), |state| async move {
                    stream_state_machine(state).await
                }
            )
        )
    }
}

async fn stream_state_machine(current: State) -> Option<(Progress, State)>{
    match current {
        State::Start(url) => {
            log::debug!("streaming url: {}", &url);
            let (tx, rx) = mpsc::channel();
            let response = reqwest::get(&url).await;
            if let Err(e) = response {
                return Some((Progress::StreamError(e.to_string()), State::Finished));
            }
            let res = response.unwrap();
            let total = res.content_length();
            let rx = Arc::new(Mutex::new(rx));
            let state = DownloadData {
                res, tx, total, downloaded: 0,
            };
            Some((Progress::Started(rx), State::Buffering(state)))}
        State::Buffering(mut state) => {
            match state.res.chunk().await {
                Err(e) => Some((Progress::StreamError(e.to_string()), State::Finished)),
                Ok(None) => Some((Progress::ToShortError, State::Finished)),
                Ok(Some(chunk)) => {
                    state.downloaded += chunk.len() as u64;
                    state.tx.send(chunk).unwrap();
                    let percentage = state.total
                        .map(|t| 100.0 * state.downloaded as f32/ t as f32)
                        .unwrap_or(0.0);  
                    let progress = Progress::Advanced(percentage);
                    Some(if state.downloaded > 4096 {
                        (progress, State::Streaming(state))
                    } else {
                        (progress, State::Buffering(state))
                    })
                }
            }
        }
        State::Streaming(mut state) => {
            match state.res.chunk().await {
                Err(e) => Some((Progress::StreamError(e.to_string()), State::Finished)),
                Ok(None) => Some((Progress::Finished, State::Finished)),
                Ok(Some(chunk)) => {
                    state.downloaded += chunk.len() as u64;
                    state.tx.send(chunk).unwrap();
                    let percentage = state.total
                        .map(|t| 100.0 * state.downloaded as f32/ t as f32)
                        .unwrap_or(0.0);
                    let progress = Progress::Advanced(percentage);
                    Some((progress, State::Streaming(state)))
                }
            }
        }
        State::Finished => {
            None
        }
    }
}

#[derive(Debug)]
pub struct DownloadData {
    res: reqwest::Response,
    tx: mpsc::Sender<bytes::Bytes>,
    total: Option<u64>,
    downloaded: u64,
}

#[derive(Debug)]
pub enum State {
    Start(String),
    Buffering(DownloadData),
    Streaming(DownloadData),
    Finished,
}
