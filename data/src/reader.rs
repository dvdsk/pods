use crate::Registration;

use super::db;
use super::subs;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task;
use tokio::task::JoinHandle;
use traits::DataUpdate;
use traits::EpisodeId;
use traits::PodcastId;

pub struct ReadReq {
    needed: Needed,
    target: Target,
}

pub enum Target {
    AllSubs,
    One(Registration),
}

impl ReadReq {
    async fn handle(&self, subs: &subs::Subs, data: &db::Store) {
        let regs = match self.target {
            Target::One(reg) => vec![reg],
            Target::AllSubs => self.needed.subs(&subs),
        };

        let data_update = self.needed.update(data);
        subs.senders.update(&regs, data_update).await;
    }

    pub fn update_all(data: Needed) -> Self {
        Self {
            needed: data,
            target: Target::AllSubs,
        }
    }

    pub(crate) fn update_one(registration: Registration, data: Needed) -> ReadReq {
        Self {
            needed: data,
            target: Target::One(registration),
        }
    }
}

#[derive(Debug)]
pub enum Needed {
    PodcastList,
    Episodes(PodcastId),
    EpisodeDetails(EpisodeId),
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
    task: JoinHandle<()>,
    tx: mpsc::Sender<ReadReq>,
}

impl Reader {
    pub(crate) fn new(data: Arc<db::Store>, subs: subs::Subs) -> Self {
        let (tx, rx) = mpsc::channel(20);
        let read_loop = read_loop(data, subs, rx);
        let task = task::spawn(read_loop);
        Self { task, tx }
    }

    pub(crate) fn read_req_tx(&self) -> mpsc::Sender<ReadReq> {
        self.tx.clone()
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.task.abort()
    }
}

async fn read_loop(data: Arc<db::Store>, subs: subs::Subs, mut rx: ReadReciever) {
    loop {
        let Some(data_req) = rx.recv().await else {
            break;
        };

        data_req.handle(&subs, &data).await;
    }
    // loop wait for new subs
    // (in future also wait for remote subs)
    // then send them the data they need
}

type ReadReciever = mpsc::Receiver<ReadReq>;
