use tracing::debug;
use traits::Forcable;
mod db;

pub struct TestState {
    config: TestConfig,
}
pub struct TestConfig {}

impl TestState {
    pub fn new() -> Self {
        TestState {
            config: TestConfig {},
        }
    }
}

impl traits::Config for TestConfig {
    fn force_remote(&mut self, _val: Option<traits::Remote>) {
        debug!("TestConfig does not store any values")
    }

    fn force_server(&mut self, _val: Option<traits::Server>) {
        debug!("TestConfig does not store any values")
    }

    fn remote(&self) -> Forcable<Option<traits::Remote>> {
        Forcable::new(None)
    }

    fn server(&self) -> Forcable<Option<traits::Server>> {
        Forcable::new(None)
    }
}

impl traits::State for TestState {
    type Config = TestConfig;

    fn config_mut(&mut self) -> &mut Self::Config {
        &mut self.config
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }
}
