use async_trait::async_trait;
use tokio::sync::mpsc;
use traits::{AppUpdate, UserIntent};

mod core;

pub struct Interface<'a> {
    pub local: Option<&'a mut InterfaceClient>,
    pub remote: &'a mut dyn traits::RemoteUI,
}

#[async_trait]
impl<'a> core::Interface for Interface<'a> {
    /// if client and remote are None block until that changes
    async fn next_intent(&mut self) -> UserIntent {
        todo!()
    }

    async fn update(&mut self, update: AppUpdate) {
        todo!()
    }
}

impl<'a> Interface<'a> {
    fn new(
        local_ui: Option<&'a mut InterfaceClient>,
        remote: &'a mut dyn traits::RemoteUI,
    ) -> Interface<'a> {
        Self {
            local: local_ui,
            remote,
        }
    }
}

type InterfaceClient = (mpsc::Receiver<UserIntent>, mpsc::Sender<AppUpdate>);

enum Reason {
    Exit,
    ConnectChange,
}

pub async fn app(
    state: impl traits::State,
    mut local_ui: Option<InterfaceClient>,
    mut remote: Box<dyn traits::RemoteUI>,
) {
    use traits::Config;

    loop {
        let server = state.config().server().get_value();
        if let Some((server, local_ui)) = server.zip(local_ui.as_mut()) {
            match core::run_remote(local_ui, server).await {
                Reason::Exit => return,
                Reason::ConnectChange => continue,
            }
        }

        let remote_config = state.config().remote().get_value();
        match remote_config {
            Some(config) => remote.enable(config),
            None => remote.disable(),
        }

        let mut interface = Interface::new(local_ui.as_mut(), remote.as_mut());
        match core::run(&mut interface).await {
            Reason::Exit => return,
            Reason::ConnectChange => continue,
        }
    }
}
