use futures::stream::FuturesUnordered;
use futures::{StreamExt, TryFutureExt};
use tokio::sync::oneshot::error::RecvError;
use tokio::sync::oneshot::Receiver;
use tokio::sync::Notify;
use tracing::instrument;

use std::future::Future;
use std::pin::Pin;
use traits::AppUpdate;

pub(crate) type Error = RecvError;
pub(crate) type Output = Result<AppUpdate, Error>;

pub(crate) struct Tasks {
    list: FuturesUnordered<Pin<Box<dyn Future<Output = Output> + Send>>>,
    notify: Notify,
}

impl Tasks {
    pub(crate) fn new() -> Self {
        Self {
            list: FuturesUnordered::new(),
            notify: Notify::new(),
        }
    }

    pub(crate) async fn next_retval(&mut self) -> AppUpdate {
        loop {
            match self.list.next().await {
                None => self.notify.notified().await,
                Some(Ok(update)) => return update,
                Some(Err(e)) => return AppUpdate::Error(e.to_string()),
            }
        }
    }

    #[instrument(skip(self, ret_rx))]
    pub(crate) fn add(&mut self, ret_rx: Receiver<AppUpdate>) {
        let task = Box::pin(ret_rx.into_future());
        let task = task as Pin<Box<dyn Future<Output = Result<AppUpdate, Error>> + Send>>;
        self.list.push(task);
        self.notify.notify_one()
    }
}
