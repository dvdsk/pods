use traits::Registration;

use std::fmt;
use tokio::sync::mpsc;
use tokio::task;
use tokio::task::AbortHandle;
use tokio::task::JoinHandle;
use tracing::instrument;
use tracing::warn;

use crate::{Needed, Subs};

use std::marker::PhantomData;

#[derive(Debug)]
pub struct ReadReq<N, C, S> {
    needed: Vec<N>,
    target: Target,
    phantom_subs: PhantomData<S>,
    phantom_ctx: PhantomData<C>,
}

#[derive(Debug)]
pub enum Target {
    AllSubs,
    One(Registration),
}

impl<N, C, S> ReadReq<N, C, S>
where
    N: Needed<C, S>,
    S: Subs + fmt::Debug, // fmt Debug bound needed to prevent ICE 
    C: fmt::Debug + Clone + Send + Sync,
{
    #[instrument(skip(data))]
    async fn handle(self, subs: &S, data: &C)
    {
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
            Target::AllSubs => needed.subs(subs),
        };

        let data_update = needed.update(&data);
        subs.senders().update(&regs, data_update).await;
    }

    // specialized version of handle that performs better on large
    // updates
    async fn handle_batch(self, subs: &S, data: C)
    {
        match self.target {
            Target::AllSubs => {
                for needed in &self.needed {
                    let data_update = needed.update(&data);
                    let regs = needed.subs(subs);
                    subs.senders().update(&regs, data_update).await;
                }
            }
            Target::One(reg) => {
                for needed in &self.needed {
                    let data_update = needed.update(&data);
                    subs.senders().update(&[reg], data_update).await;
                }
            }
        }
    }

    pub fn update_all(data: Vec<N>) -> Self {
        Self {
            needed: data,
            target: Target::AllSubs,
            phantom_ctx: PhantomData,
            phantom_subs: PhantomData,
        }
    }

    pub(crate) fn update_one(registration: Registration, data: N) -> ReadReq<N, C, S> {
        Self {
            needed: vec![data],
            target: Target::One(registration),
            phantom_ctx: PhantomData,
            phantom_subs: PhantomData,
        }
    }
}

pub struct Reader<N, C, S> {
    tx: mpsc::Sender<ReadReq<N, C, S>>,
    abort_handle: AbortHandle,
}

impl<N, C, S> Reader<N, C, S>
where
    N: Needed<C, S> + Send + Sync + 'static,
        S: Subs + 'static,
        C: Send + Sync + Clone + fmt::Debug +'static,
{
    #[must_use]
    pub fn new(data: C, subs: S) -> (Self, JoinHandle<()>)
    {
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

    pub(crate) fn read_req_tx(&self) -> mpsc::Sender<ReadReq<N, C, S>> {
        self.tx.clone()
    }
}

impl<N, C, S> Drop for Reader<N, C, S> {
    fn drop(&mut self) {
        self.abort_handle.abort()
    }
}

async fn read_loop<S, C, N>(data: C, subs: S, mut rx: ReadReciever<N, C, S>)
where
    N: Needed<C, S>,
    S: Subs,
    C: Send + Sync + Clone + fmt::Debug,
{
    loop {
        let Some(data_req) = rx.recv().await else {
            break;
        };

        data_req.handle(&subs, &data).await;
    }
    warn!("Read loop shutting down, can no longer read data")
}

type ReadReciever<N, C, S> = mpsc::Receiver<ReadReq<N, C, S>>;
