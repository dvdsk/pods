// mod reader;
mod publisher;
mod sub;

use std::any::Any;
use std::marker::PhantomData;

use sub::{Clients, Senders, Subscription};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use traits::Registration;

#[derive(Clone)]
pub struct Publisher<U, K>
where
    K: Clone + Send,
    U: Clone + Send,
{
    clients: Clients<K>,
    senders: Senders<U>,
    update_queue: mpsc::Sender<(K, publisher::Target)>,
    update_ty: PhantomData<U>,
    key_ty: PhantomData<K>,
}

pub struct PublishTask {
    task: JoinHandle<()>,
}

type PanicReason = Box<dyn Any + Send + 'static>;
impl PublishTask {
    #[must_use]
    pub async fn watch_for_errs(self) -> PanicReason {
        self.task
            .await
            .expect_err("publisher is never canceld")
            .try_into_panic()
            .expect("publisher is never canceld")
    }
}

pub trait AsKey<K> {
    fn as_key(&self) -> K;
}

impl<U, K> Publisher<U, K>
where
    U: AsKey<K> + Clone + Send + Sync + 'static,
    K: Clone + Send + Eq + std::hash::Hash + PartialEq + 'static,
{
    #[must_use]
    pub fn new<F>(update_source: F) -> (Self, PublishTask)
    where
        F: FnMut(&K) -> U + Send + 'static,
    {
        let (update_tx, update_rx) = mpsc::channel(30);
        let publisher = Self {
            clients: Clients::default(),
            senders: Senders::default(),
            update_ty: PhantomData,
            key_ty: PhantomData,
            update_queue: update_tx,
        };
        let work = publisher::work(publisher.clone(), update_source, update_rx);
        let task = tokio::task::spawn(work);
        (publisher, PublishTask { task })
    }
    pub fn publish(&self, update: &U) {
        let msg = (update.as_key(), publisher::Target::All);
        self.update_queue.try_send(msg).unwrap()
    }
    // pub fn publish_batch(&self, update: &[&U]) {
    //     let msg = (update.key(), publisher::Target::All);
    //     self.update_queue.try_send(msg).unwrap()
    // }
    #[must_use]
    pub fn subscribe(&self, reg: Registration, key: impl Into<K>) -> Subscription {
        let key = key.into();
        let sub = self.clients.sub(reg, key.clone());
        let msg = (key, publisher::Target::NewSub { reg });
        self.update_queue.try_send(msg).unwrap();
        sub
    }
    pub fn register(&self, tx: mpsc::Sender<U>, description: &'static str) -> Registration {
        let id = self.senders.add(tx);
        Registration::new(id, description)
    }
}
