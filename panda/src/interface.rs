use traits::eyre;
use async_trait::async_trait;

#[derive(Debug)]
pub struct Unified<'a> {
    updater: Updater<'a>,
    intent: IntentReciever<'a>,
    remote_controller: &'a mut dyn traits::RemoteController,
}

#[derive(Debug)]
pub struct Updater<'a> {
    remote_tx: &'a mut dyn traits::Updater,
    local_tx: &'a mut dyn traits::Updater,
}

#[async_trait]
impl<'a> traits::Updater for Updater<'a> {
    async fn update(&mut self, msg: traits::AppUpdate) -> Result<(), eyre::Report> {
        let send_local = self.local_tx.update(msg.clone());
        let send_remote = self.remote_tx.update(msg);
        tokio::try_join!(send_remote, send_local)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct IntentReciever<'a> {
    remote_rx: &'a mut dyn traits::IntentReciever,
    local_rx: &'a mut dyn traits::IntentReciever,
}

#[async_trait]
impl<'a> traits::IntentReciever for IntentReciever<'a> {
    async fn next_intent(&mut self) -> Option<traits::UserIntent> {
        let reveive_local = self.local_rx.next_intent();
        let reveive_remote = self.remote_rx.next_intent();
        tokio::select!(
            res = reveive_remote => res,
            res = reveive_local => res,
        )
    }
}

impl<'a> Unified<'a> {
    pub fn new(
        local: &'a mut Box<dyn traits::LocalUI>,
        remote: &'a mut Box<dyn traits::RemoteUI>,
    ) -> Unified<'a> {
        let (local_tx, local_rx) = local.ports();
        let (remote_tx, remote_rx, remote_controller) = remote.ports();
        Self {
            updater: Updater {
                remote_tx,
                local_tx,
            },
            intent: IntentReciever {
                remote_rx,
                local_rx,
            },
            remote_controller,
        }
    }
}

#[async_trait]
impl<'a> traits::RemoteUI for Unified<'a> {
    fn ports(
        &mut self,
    ) -> (
        &mut dyn traits::Updater,
        &mut dyn traits::IntentReciever,
        &mut dyn traits::RemoteController,
    ) {
        (
            &mut self.updater,
            &mut self.intent,
            self.remote_controller,
        )
    }

    fn controller(&mut self) -> &mut dyn traits::RemoteController {
        self.remote_controller
    }
}
