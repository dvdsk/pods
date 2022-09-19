use async_trait::async_trait;
use traits::{AppUpdate, UserIntent};

mod core;

pub struct Interface<'a, 'b> {
    pub local: &'a mut Option<&'b mut dyn traits::LocalUI>,
    pub remote: &'a mut Box<dyn traits::RemoteUI>,
}

#[async_trait]
impl<'a, 'b> core::Interface for Interface<'a, 'b> {
    /// if client and remote are None block until that changes
    async fn next_intent(&mut self) -> UserIntent {
        todo!()
    }

    async fn update(&mut self, update: AppUpdate) {
        todo!()
    }
}

impl<'a, 'b: 'a> Interface<'a, 'b> {
    fn new(
        local_ui: &'a mut Option<&'b mut dyn traits::LocalUI>,
        remote: &'a mut Box<dyn traits::RemoteUI>,
    ) -> Interface<'a, 'b> {
        Self {
            local: local_ui,
            remote,
        }
    }
}

enum Reason {
    Exit,
    ConnectChange,
}

trait OptionBoxAsMut {
    fn inner_as_mut(&mut self) -> Option<&mut dyn traits::LocalUI>;
}

impl OptionBoxAsMut for Option<Box<dyn traits::LocalUI>> {
    fn inner_as_mut(&mut self) -> Option<&mut dyn traits::LocalUI> {
        match self {
            Some(b) => Some(b.as_mut()),
            None => None,
        }
    }
}

pub async fn app(
    state: impl traits::State,
    mut local_ui: Option<Box<dyn traits::LocalUI>>,
    mut remote: Box<dyn traits::RemoteUI>,
) {
    use traits::Config as _;

    let mut local_ui = local_ui.inner_as_mut();

    // only allow starting of remote if
    // we are not connecting to a remote
    let server = state.config().server().get_value();
    if server.is_none() {
        let remote_config = state.config().remote().get_value();
        match remote_config {
            Some(config) => remote.enable(config),
            None => remote.disable(),
        }
    }

    loop {
        let server = state.config().server().get_value();
        match (server, local_ui.as_mut()) {
            (Some(server), Some(local_ui)) => match core::run_remote(*local_ui, server).await {
                Reason::Exit => break,
                Reason::ConnectChange => continue,
            },
            _ => (),
        }

        let mut interface = Interface::new(&mut local_ui, &mut remote);
        match core::run(&mut interface).await {
            Reason::Exit => break,
            Reason::ConnectChange => continue,
        }
    }
}
