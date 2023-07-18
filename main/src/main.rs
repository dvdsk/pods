use std::sync::Arc;

use clap::Parser;
use data::Data;
use presenter::InternalPorts;
use presenter::Ui;
use tokio::sync::Mutex;
use traits::DataRStore;
use traits::Settings as _;
// use traits::State as _;

use tokio::signal;

mod errors;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum UiArg {
    Gui,
    Tui,
    None,
}

#[derive(Parser, Debug)]
#[clap(long_about = "")]
struct Cli {
    #[arg(long, default_value("tui"))]
    ui: UiArg,

    #[arg(group = "remote")]
    connect_to: Option<u64>,
    #[arg(requires = "remote")]
    password: Option<String>,

    #[arg(long)]
    server: bool,
    #[arg(long)]
    server_password: Option<String>,
    #[arg(long)]
    server_port: Option<u16>,
}

fn force_cli_arguments(config: &mut impl traits::Settings, cli: &Cli) {
    use traits::Remote;
    use traits::Server;

    let remote = cli.connect_to.map(|id| Remote {
        id,
        password: cli.password.clone(),
    });
    config.force_remote(remote);
    let server = match (cli.server, cli.server_port, cli.server_password.clone()) {
        (true, port, password) => Some(Server { port, password }),
        (false, None, None) => None,
        (false, None, password @ Some(_)) => Some(Server {
            port: None,
            password,
        }),
        (false, port @ Some(_), None) => Some(Server {
            port,
            password: None,
        }),
        (false, port @ Some(_), password @ Some(_)) => Some(Server { port, password }),
    };
    config.force_server(server);
}

#[tokio::main]
async fn main() {
    errors::set_error_hook();
    errors::install_tracing();

    let cli = Cli::parse();
    let mut state = Data::new();
    force_cli_arguments(state.settings_mut(), &cli);

    let (ui_runtime, ui_port) = match cli.ui {
        UiArg::None => (None, None),
        choice @ UiArg::Tui | choice @ UiArg::Gui => {
            let ui_fn = match choice {
                UiArg::None => unreachable!(),
                UiArg::Gui => Box::new(gui::new) as Box<dyn Fn(InternalPorts) -> Box<dyn Ui>>,
                UiArg::Tui => Box::new(tui::new) as Box<dyn Fn(InternalPorts) -> Box<dyn Ui>>,
            };
            let (runtime, interface) = presenter::new(ui_fn);
            (Some(runtime), Some(interface))
        }
    };

    let server_config = state.settings().server().get_value();

    let data = Box::new(state) as Box<dyn traits::DataStore>;
    let remote = Box::new(remote_ui::new(server_config));
    let searcher = Arc::new(Mutex::new(
        Box::new(search::new()) as Box<dyn traits::IndexSearcher>
    ));
    tokio::task::spawn(panda::app(data, ui_port, remote, searcher));

    match ui_runtime {
        Some(mut ui) => ui.run().await.unwrap(),
        None => signal::ctrl_c().await.unwrap(),
    }
}
