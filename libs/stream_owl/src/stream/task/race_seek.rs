use futures::Future;
use tokio::sync::mpsc;
use tracing::debug;

#[derive(Debug)]
pub(super) struct StreamCanceldTomato;

use super::Cmd;

/// Waits for an pa
pub(super) async fn receive_pos_and_process_pause(
    cmd_rx: &mut mpsc::Receiver<Cmd>,
) -> Result<u64, StreamCanceldTomato> {
    let mut paused = false;
    let mut seek = None;

    let pos = loop {
        let Some(cmd) = cmd_rx.recv().await else {
            return Err(StreamCanceldTomato);
        };

        match cmd {
            Cmd::Seek(pos) if !paused => break pos,
            Cmd::Seek(pos) if paused => {
                debug!("storing seek, stream is paused");
                seek = Some(pos);
            }
            Cmd::Pause => paused = true,
            Cmd::Resume => paused = false,
        }

        if let Some(pos) = seek {
            break pos;
        }
    };

    Ok(seek)
}

/// work on future until we get pause, then stop working on future until we get resume. If we get
/// seek store it until resume
pub(super) async fn join_with_pause(
    cmd_rx: &mut mpsc::Receiver<Cmd>,
    future: impl Future,
) -> Result<u64, StreamCanceldTomato> {
    let mut paused = false;
    let mut seek = None;

    let pos = loop {
    };

    Ok(seek)
}
