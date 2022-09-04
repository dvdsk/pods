use std::error::Error;
use std::path::Path;

pub enum UserIntent {
    Exit,
}

pub enum AppUpdate {
    Exit
}

pub trait Ui {
    fn run(&mut self) -> Result<(), Box<dyn Error>>; 
}

pub trait Db : Sized {
    type Error;
    fn open(path: &Path) -> Result<Self, Self::Error>;
}
