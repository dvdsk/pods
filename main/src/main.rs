use std::path::Path;
use traits::{Ui, Db};
use clap::Parser;

#[derive(Debug, Clone, clap::ValueEnum)]
enum UiType {
    Gui,
    Tui,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Ui to use
    #[arg(short, long)]
    ui: UiType,
}


fn main() {
    let args = Args::parse();

    let mut gui: Box<dyn Ui> = match args.ui {
        UiType::Gui => Box::new(gui::new()),
        UiType::Tui => Box::new(tui::new()),
    };
    gui.run().unwrap();
    let db = db::DerivedDb::open(Path::new("database.db"));

    


    println!("Hello, world!");


}
