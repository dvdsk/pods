use clap::{Parser, Subcommand};
use state::State;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use traits::ClientInterface;
use traits::Config as _;
use traits::State as _;

use tokio::signal;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum UiChoice {
    Gui,
    Tui,
    None,
}

#[derive(Parser, Debug)]
#[clap(long_about = "")]
struct Cli {
    #[clap(long, default_value_t: UiChoice::Tui)]
    ui: UiChoice,
    #[clap(group = "remote")]
    connect_to: Option<u64>,
    #[clap(requires = "remote")]
    password: Option<String>,
    server: Option<String>,
}

fn force_cli_arguments(config: &mut impl traits::Config, cli: &Cli) {
    config.force_remote(todo!());
    config.force_server(todo!());
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let state = State::new();
    force_cli_arguments(state.config_mut(), &cli);

    let (ui_runtime, ui_interface) = match cli.ui {
        UiChoice::Gui => todo!(),
        UiChoice::Tui => todo!(),
        UiChoice::None => (None, None),
    };

    let remote = state.config()
        .remote()
        .get_value()
        .map(|remote| todo!("start remote listener"));

    let interface = panda::Interface {
        client: ui_interface,
        remote: todo!(),
    };

    thread::spawn(move || panda::run(interface));

    match ui_runtime {
        Some(ui) => ui.run().unwrap(),
        None => signal::ctrl_c().await.unwrap(),
    }
}
