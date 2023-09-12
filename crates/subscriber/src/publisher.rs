use tokio::sync::mpsc;
use traits::Registration;

use crate::{Order, Publisher};

pub(super) enum Target {
    NewSub { reg: Registration },
    All,
}

pub(crate) async fn work<F, U, K>(
    publisher: Publisher<U, K>,
    mut update_source: F,
    mut update_tx: mpsc::Receiver<Order<K>>,
) where
    F: FnMut(&K) -> U,
    U: Clone + Send + Sync,
    K: Clone + Send + Eq + PartialEq + std::hash::Hash,
{
    while let Some(order) = update_tx.recv().await {
        match order {
            Order::Inform { key, reg } => {
                let update = update_source(&key);
                publisher.senders.update(&[reg], update).await;
            }
            Order::Publish { updated_keys } => {
                for key in updated_keys {
                    let update = update_source(&key);
                    let recievers = publisher.clients.regs(&key);
                    publisher.senders.update(&recievers, update).await;
                }
            }
        }
    }
}
