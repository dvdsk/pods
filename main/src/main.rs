use clap::Parser;
use presenter::Interface;
use presenter::Ui;
use state::State;
use std::thread;
use traits::ClientInterface;
use traits::Config as _;
use traits::State as _;

use tokio::signal;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum UiArg {
    Gui,
    Tui,
    None,
}

#[derive(Parser, Debug)]
#[clap(long_about = "")]
struct Cli {
    #[arg(long, default_value("Tui"))]
    ui: UiArg,

    #[arg(group = "remote")]
    connect_to: Option<u64>,
    #[arg(requires = "remote")]
    password: Option<String>,

    server: bool,
    server_password: Option<String>,
    server_port: Option<u16>,
}

fn force_cli_arguments(config: &mut impl traits::Config, cli: &Cli) {
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
    let cli = Cli::parse();
    let mut state = State::new();
    force_cli_arguments(state.config_mut(), &cli);

    let (ui, presenter) = match cli.ui {
        UiArg::None => (None, None),
        choice @ UiArg::Tui | choice @ UiArg::Gui => {
            let ui_fn = match choice {
                UiArg::None => unreachable!(),
                UiArg::Gui => Box::new(gui::new) as Box<dyn Fn(Interface) -> Box<dyn Ui>>,
                UiArg::Tui => Box::new(tui::new) as Box<dyn Fn(Interface) -> Box<dyn Ui>>,
            };
            let (runtime, presenter) = presenter::new(ui_fn);
            let presenter = Box::new(presenter) as Box<dyn ClientInterface>;
            (Some(runtime), Some(presenter))
        }
    };

    let remote = state
        .config()
        .remote()
        .get_value()
        .map(|remote| todo!("start remote listener"));

    let interface = panda::Interface {
        client: presenter,
        remote,
    };

    thread::spawn(move || panda::run(interface));

    match ui {
        Some(mut ui) => ui.run().unwrap(),
        None => signal::ctrl_c().await.unwrap(),
    }
}
