mod home;
mod episodes;
mod errorpage;

pub use home::Home;
pub use episodes::Episodes;
pub use errorpage::errorpage;

pub enum Page{
    Home,
    Episodes,
}
