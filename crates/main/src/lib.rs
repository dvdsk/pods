use std::panic;
use std::sync::Arc;

use presenter::Ui;
use tokio::sync::Mutex;
use tokio::task::JoinError;
use tokio::signal;
use tracing::info;
use traits::{DataStore, Feed, IndexSearcher, LocalUI};

use futures::FutureExt;
use futures_concurrency::future::Race;

// #[cfg(testing)]
pub mod testing;

enum Res {
    Data(Result<(), JoinError>),
    App(Result<(), JoinError>),
    UI(Result<(), color_eyre::Report>),
    CtrlC(Result<(), std::io::Error>),
}

pub async fn run_and_watch_for_errors(
    data: Box<dyn DataStore>,
    ui_port: Option<Box<dyn LocalUI>>,
    remote: Box<remote_ui::Interface>,
    searcher: Arc<Mutex<dyn IndexSearcher>>,
    feed: Box<dyn Feed>,
    data_maintain: tokio::task::JoinHandle<()>,
    ui_runtime: Option<Box<dyn Ui>>,
) {
    let app = tokio::task::spawn(panda::app(data, ui_port, remote, searcher, feed)).map(Res::App);
    let data_maintain = data_maintain.map(Res::Data);

    let res = match ui_runtime {
        Some(mut ui) => {
            let ui = ui.run().map(Res::UI);
            (app, data_maintain, ui).race().await
        }
        None => {
            let ui = signal::ctrl_c().map(Res::CtrlC);
            (app, data_maintain, ui).race().await
        }
    };

    match res {
        Res::App(Err(e)) => {
            let reason = e.try_into_panic().expect("app is never canceld");
            panic::resume_unwind(reason);
        }
        Res::Data(Err(e)) => {
            let reason = e.try_into_panic().expect("data maintain is never canceld");
            panic::resume_unwind(reason);
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
