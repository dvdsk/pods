pub enum UserIntent {
    Exit,
}

#[derive(Debug)]
pub enum AppUpdate {
    Exit,
}

pub struct Forcable<T: Sized> {
    forced: bool,
    value: T,
}

impl<T: Sized> Forcable<T> {
    pub fn get_value(self) -> T {
        self.value
    }
    pub fn is_forced(&self) -> bool {
        self.forced
    }
}

pub trait State: Sized {
    type Config;
    fn config_mut(&mut self) -> &mut Self::Config;
    fn config(&self) -> &Self::Config;
}

pub struct Remote {
    pub id: u64,
    pub password: Option<String>,
}

pub struct Server {
    pub port: Option<u16>,
    pub password: Option<String>,
}

pub trait Config: Sized {
    fn remote(&self) -> Forcable<Option<Remote>>;
    fn server(&self) -> Forcable<Option<Server>>;
    fn force_remote(&mut self, val: Option<Remote>);
    fn force_server(&mut self, val: Option<Server>);
}

pub trait ClientInterface : Send {
    fn update(&mut self, msg: AppUpdate);
    fn next_intent(&mut self) -> UserIntent;
}

