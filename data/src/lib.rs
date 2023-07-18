mod config;
mod db;

use config::Settings;

pub struct Data {
    config: Settings,
}

impl Data {
    pub fn new() -> Self {
        Data {
            config: Settings {},
        }
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.config
    }
}

impl traits::DataRStore for Data {
    fn get_podcasts(&self) -> Box<dyn traits::DataSub> {
        todo!()
    }

    fn settings(&self) -> &dyn traits::Settings {
        &self.config
    }
}

impl traits::DataWStore for Data {
    fn update_podcasts(&mut self) {
        todo!()
    }

    fn sub_podcasts(&mut self) {
        todo!()
    }
}

impl traits::LocalOrRemoteStore for Data {
    fn set_remote(&mut self) {
        todo!()
    }

    fn set_local(&mut self) {
        todo!()
    }
}

impl traits::DataRWStore for Data {}
impl traits::DataStore for Data {}
