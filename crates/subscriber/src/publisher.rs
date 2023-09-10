use tokio::sync::mpsc;
use traits::Registration;

use crate::Publisher;

pub(super) enum Target {
    NewSub { reg: Registration },
    All,
}

pub(crate) async fn work<F, U, K>(
    publisher: Publisher<U, K>,
    mut update_source: F,
    mut update_tx: mpsc::Receiver<(K, Target)>,
) where
    F: FnMut(&K) -> U,
    U: Clone + Send + Sync,
    K: Clone + Send + Eq + PartialEq + std::hash::Hash,
{
    while let Some((key, target)) = update_tx.recv().await {
        let update = update_source(&key);
        let recievers = match target {
            Target::NewSub { reg } => vec![reg],
            Target::All => publisher.clients.regs(&key),
        };

        publisher.senders.update(&recievers, update).await;
    }
}
