use crate::Registration;

use super::db;
use super::subs;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task;
use tokio::task::AbortHandle;
use tokio::task::JoinHandle;
use tracing::instrument;
use tracing::warn;
use traits::DataUpdate;
use traits::PodcastId;

#[derive(Debug)]
pub struct ReadReq {
    needed: Vec<Needed>,
    target: Target,
}

#[derive(Debug)]
pub enum Target {
    AllSubs,
    One(Registration),
}

impl ReadReq {
    #[instrument(skip(data))]
    async fn handle(self, subs: &subs::Subs, data: &Arc<db::Store>) {
        let needed = if self.needed.len() > 1 {
            /* TODO: 
             * set/set compare for batches <27-08-23, dvdsk> */
            self.handle_batch(subs.clone(), data.clone()).await;
            return;
        } else {
            self.needed.first().unwrap()
        };

        let regs = match self.target {
            Target::One(reg) => vec![reg],
            Target::AllSubs => needed.subs(&subs),
        };

        let data_update = needed.update(&data);
        subs.senders.update(dbg!(&regs), data_update).await;
    }

    // specialized version of handle that performs better on large
    // updates
    async fn handle_batch(self, subs: subs::Subs, data: Arc<db::Store>) {
        match self.target {
            Target::AllSubs => {
                for needed in &self.needed {
                    let data_update = needed.update(&data);
                    let regs = needed.subs(&subs);
                    subs.senders.update(&regs, data_update).await;
                }
            }
            Target::One(reg) => {
                for needed in &self.needed {
                    let data_update = needed.update(&data);
                    subs.senders.update(&[reg], data_update).await;
                }
            }
        }
    }

    pub fn update_all(data: Vec<Needed>) -> Self {
        Self {
            needed: data,
            target: Target::AllSubs,
        }
    }

    pub(crate) fn update_one(registration: Registration, data: Needed) -> ReadReq {
        Self {
            needed: vec![data],
            target: Target::One(registration),
        }
    }
}

#[derive(Debug)]
pub enum Needed {
    PodcastList,
    Episodes(PodcastId),
    EpisodeDetails(PodcastId),
}

impl Needed {
    fn subs(&self, subs: &subs::Subs) -> Vec<Registration> {
        match self {
            Needed::PodcastList => subs.podcast.regs(),
            Needed::Episodes(podcast_id) => subs.episodes.regs(podcast_id),
            Needed::EpisodeDetails(episode_id) => subs.episode_details.regs(episode_id),
        }
    }

    fn update(&self, data: &db::Store) -> DataUpdate {
        match self {
            Needed::PodcastList => data.podcast_update(),
            Needed::Episodes(podcast_id) => data.episodes_update(*podcast_id),
            Needed::EpisodeDetails(episode_id) => data.episode_details_update(*episode_id),
        }
    }
}

pub(crate) struct Reader {
    tx: mpsc::Sender<ReadReq>,
    abort_handle: AbortHandle,
}

impl Reader {
    #[must_use]
    pub(crate) fn new(data: Arc<db::Store>, subs: subs::Subs) -> (Self, JoinHandle<()>) {
        let (tx, rx) = mpsc::channel(20);
        let read_loop = read_loop(data, subs, rx);
        let task = task::spawn(read_loop);
        (
            Self {
                tx,
                abort_handle: task.abort_handle(),
            },
            task,
        )
    }

    pub(crate) fn read_req_tx(&self) -> mpsc::Sender<ReadReq> {
        self.tx.clone()
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.abort_handle.abort()
    }
}

async fn read_loop(data: Arc<db::Store>, subs: subs::Subs, mut rx: ReadReciever) {
    loop {
        let Some(data_req) = rx.recv().await else {
            break;
        };

        data_req.handle(&subs, &data).await;
    }
    warn!("Read loop shutting down, can no longer read data")
}

type ReadReciever = mpsc::Receiver<ReadReq>;
