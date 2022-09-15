use traits::{AppUpdate, UserIntent, ClientInterface};

pub struct Interface {
    pub client: Option<Box<dyn ClientInterface>>,
    pub remote: Option<()>, // TODO
}

impl Interface {
    fn allow_remote(&mut self, allowed: bool) {
        todo!()
    }

    fn next_intent(&mut self) -> UserIntent {
        todo!()
    }

    fn update(&mut self, update: AppUpdate) {
        todo!()
    }
}

pub fn run(mut interface: Interface) {
    loop {
        // get reciever for all the clients

        // join on reciever?
        match interface.next_intent() {
            UserIntent::Exit => {
                interface.update(AppUpdate::Exit);
                return
            }
        }
    }
}
