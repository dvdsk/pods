use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tracing::instrument;
use traits::Registration;

#[derive(Debug)]
pub(crate) struct Client {
    expired: Arc<AtomicBool>,
    registration: Registration,
}

impl Client {
    fn new(registration: Registration) -> (Self, Subscription) {
        let client = Self {
            expired: Arc::new(AtomicBool::new(false)),
            registration,
        };
        let sub = Subscription {
            expired: client.expired.clone(),
        };
        (client, sub)
    }

    pub fn not_expired(&self) -> bool {
        !self.expired.load(Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub struct Subscription {
    expired: Arc<AtomicBool>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.expired.store(true, Ordering::Relaxed);
    }
}

impl traits::DataSub for Subscription {}

#[derive(Debug, Clone)]
pub struct Clients<K>(Arc<Mutex<HashMap<K, Vec<Client>>>>);

impl<K> Default for Clients<K> {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
}

impl<K> Clients<K>
where
    K: Eq + PartialEq + std::hash::Hash,
{
    pub fn sub(&self, registration: Registration, id: K) -> Subscription {
        let mut map = self.0.lock().unwrap();
        let (client, sub) = Client::new(registration);
        if let Some(clients) = map.get_mut(&id) {
            clients.push(client)
        } else {
            let clients = vec![client];
            map.insert(id, clients);
        }
        sub
    }

    pub fn regs(&self, id: &K) -> Vec<Registration> {
        let mut map = self.0.lock().unwrap();
        let Some(list) = map.get_mut(id) else {
            return Vec::new();
        };
        list.retain(Client::not_expired);
        list.iter().map(|c| c.registration).collect()
    }
}

#[derive(Clone)]
pub struct Senders<U>(Arc<Mutex<Vec<mpsc::Sender<U>>>>);

impl<U> Default for Senders<U> {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}

impl<U: Clone> Senders<U> {
    pub fn add(&self, client: mpsc::Sender<U>) -> usize {
        let mut list = self.0.lock().unwrap();
        list.push(client);
        list.len() - 1
    }
    #[instrument(skip(self, update))]
    pub async fn update(&self, recievers: &[Registration], update: U) {
        for reciever in recievers {
            let tx = {
                let list = self.0.lock().unwrap();
                list[reciever.id()].clone()
            };
            tx.send(update.clone()).await.unwrap();
        }
    }
}
