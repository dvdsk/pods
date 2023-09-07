use traits::Source;

pub struct Player {}

impl Player {
    pub fn new() -> Self {
        Player {}
    }
}

impl traits::Player for Player {
    fn play(&mut self, source: Box<dyn Source>) {
        todo!()
    }
    fn pause(&mut self) {
        todo!()
    }
    fn stop(&mut self) {
        todo!()
    }
    fn seek(&mut self) {
        todo!()
    }
}
