use clap::Parser;
use presenter::Interface;
use presenter::Ui;
use state::TestState;
use traits::State as _;
use traits::Config as _;

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
    let mut state = TestState::new();
    force_cli_arguments(state.config_mut(), &cli);

    let (ui, ui_client) = match cli.ui {
        UiArg::None => (None, None),
        choice @ UiArg::Tui | choice @ UiArg::Gui => {
            let ui_fn = match choice {
                UiArg::None => unreachable!(),
                UiArg::Gui => Box::new(gui::new) as Box<dyn Fn(Interface) -> Box<dyn Ui>>,
                UiArg::Tui => Box::new(tui::new) as Box<dyn Fn(Interface) -> Box<dyn Ui>>,
            };
            let (runtime, user_intent, app_updates) = presenter::new(ui_fn);
            (Some(runtime), Some((user_intent, app_updates)))
        }
    };

    let remote_config = state
        .config()
        .remote()
        .get_value();

    let remote = Box::new(remote_ui::new(remote_config));
    tokio::task::spawn(panda::app(state, ui_client, remote));

    match ui {
        Some(mut ui) => ui.run().await.unwrap(),
        None => signal::ctrl_c().await.unwrap(),
    }
}
