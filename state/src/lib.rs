mod db;

pub struct State {}
pub struct Config {}

impl State {
    pub fn new() -> Self {
        State {}
    }
}

impl traits::Config for Config {

    fn force_remote(&mut self, val: Option<traits::Remote>) {
        todo!()
    }

    fn force_server(&mut self, val: Option<traits::Server>) {
        todo!()
    }

    fn remote(&self) -> traits::Forcable<Option<traits::Remote>> {
        todo!()
    }

    fn server(&self) -> traits::Forcable<Option<traits::Server>> {
        todo!()
    }

}

impl traits::State for State {
    type Config = Config;

    fn config_mut(&mut self) -> &mut Self::Config {
        todo!()
    }

    fn config(&self) -> &Self::Config {
        todo!()
    }
}
