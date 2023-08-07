use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use traits::{DataUpdate, Registration};

#[derive(Debug)]
pub(crate) struct Client {
    expired: Arc<AtomicBool>,
    registration: Registration,
}

impl Client {
    fn new(registration: Registration) -> Self {
        Self {
            expired: Arc::new(AtomicBool::new(false)),
            registration,
        }
    }

    pub fn not_expired(&self) -> bool {
        !self.expired.load(Ordering::Relaxed)
    }
}

pub struct Sub {
    expired: Arc<AtomicBool>,
}

impl Drop for Sub {
    fn drop(&mut self) {
        self.expired.store(true, Ordering::Relaxed);
    }
}

impl traits::DataSub for Sub {}

#[derive(Debug, Default, Clone)]
pub(crate) struct Clients(Arc<Mutex<Vec<Client>>>);

impl Clients {
    pub fn sub(&self, registration: Registration) -> Sub {
        let client = Client::new(registration);
        let sub = Sub {
            expired: client.expired.clone(),
        };
        self.0.lock().unwrap().push(client);
        sub
    }

    pub(crate) fn regs(&self) -> Vec<Registration> {
        let mut list = self.0.lock().unwrap();
        list.retain(Client::not_expired);
        list.iter().map(|c| c.registration).collect()
    }
}

#[derive(Default, Clone)]
pub struct Senders(Arc<Mutex<Vec<Box<dyn traits::DataTx>>>>);

impl Senders {
    fn add(&self, client: Box<dyn traits::DataTx>) -> usize {
        let mut list = self.0.lock().unwrap();
        list.push(client);
        list.len() - 1
    }
    pub(super) async fn update(&self, recievers: &[Registration], update: DataUpdate) {
        for reciever in recievers {
            let mut tx = {
                let list = self.0.lock().unwrap();
                list[reciever.id()].box_clone()
            };
            tx.send(update.clone()).await;
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct Subs {
    pub(crate) senders: Senders,
    pub(crate) podcast: Clients,
}

macro_rules! sub {
    ($name:ident, $member:ident) => {
        pub fn $name(&self, registration: Registration) -> Sub {
            self.$member.sub(registration)
        }
    };
}

impl Subs {
    pub(crate) fn register(&self, client: Box<dyn traits::DataTx>) -> Registration {
        let idx = self.senders.add(client);
        Registration::new(idx)
    }
    sub! {sub_podcasts, podcast}
}
