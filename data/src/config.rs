use tracing::debug;
use traits::Forcable;

pub struct Settings {}

impl traits::Settings for Settings {
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
