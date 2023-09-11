use std::panic;
use std::sync::Arc;

use presenter::Ui;
use subscriber::PublishTask;

use tokio::signal;
use tokio::sync::Mutex;
use tokio::task::JoinError;
use tracing::info;
use traits::{DataStore, Feed, IndexSearcher, LocalUI};

use futures::FutureExt;
use futures_concurrency::future::Race;

// #[cfg(testing)]
pub mod testing;

enum Res {
    Data(Box<dyn std::any::Any + Send + 'static>),
    App(Result<(), JoinError>),
    UI(Result<(), color_eyre::Report>),
    CtrlC(Result<(), std::io::Error>),
    Media(Box<dyn std::any::Any + Send + 'static>),
}

pub async fn run_and_watch_for_errors(
    data: Box<dyn DataStore>,
    ui_port: Option<Box<dyn LocalUI>>,
    remote: Box<remote_ui::Interface>,
    searcher: Arc<Mutex<dyn IndexSearcher>>,
    media: media::Media,
    mut media_handle: media::Handle,
    player: Box<dyn traits::Player>,
    feed: Box<dyn Feed>,
    data_maintain: PublishTask,
    ui_runtime: Option<Box<dyn Ui>>,
) {
    let app = tokio::task::spawn(panda::app(
        data,
        ui_port,
        remote,
        searcher,
        Box::new(media),
        player,
        feed,
    ))
    .map(Res::App);
    let data_maintain = data_maintain.watch_for_errs().map(Res::Data);
    let media_handle = media_handle.errors().map(Res::Media);

    let res = match ui_runtime {
        Some(mut ui) => {
            let ui = ui.run().map(Res::UI);
            (app, data_maintain, media_handle, ui).race().await
        }
        None => {
            let ui = signal::ctrl_c().map(Res::CtrlC);
            (app, data_maintain, media_handle, ui).race().await
        }
    };

    match res {
        Res::App(Err(e)) => {
            let reason = e.try_into_panic().expect("app is never canceld");
            panic::resume_unwind(reason);
        }
        Res::Media(p) | Res::Data(p) => {
            panic::resume_unwind(p);
        }
        Res::UI(Err(e)) => panic!("UI crashed, reason: {e:?}"),
        Res::CtrlC(Err(e)) => {
            panic!("could not wait for ctrl+C: {}", e);
        }
        _ => {
            info!("Exiting");
            return;
        }
    }
}
