use tracing::instrument;

mod core;
mod interface;

#[derive(Debug)]
enum Reason {
    Exit,
    ConnectChange,
}

#[instrument(skip_all)]
pub async fn app(
    state: impl traits::State,
    mut local_ui: Option<Box<dyn traits::LocalUI>>,
    mut remote: Box<dyn traits::RemoteUI>,
) {
    use traits::Config as _;

    loop {
        dbg!();
        let server = state.config().server().get_value();
        match (server, local_ui.as_mut()) {
            (Some(server), Some(local_ui)) => {
                match core::run_remote(local_ui.as_mut(), server).await {
                    Reason::Exit => break,
                    Reason::ConnectChange => continue,
                }
            }
            _ => (),
        }

        match local_ui {
            None => match core::run(remote.as_mut()).await {
                Reason::Exit => break,
                Reason::ConnectChange => unreachable!(),
            },
            Some(ref mut local_ui) => {
                let mut interface = interface::Unified::new(local_ui, &mut remote);
                match core::run(&mut interface).await {
                    Reason::Exit => break,
                    Reason::ConnectChange => continue,
                }
            }
        }
    }
}
