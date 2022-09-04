use std::sync::mpsc::{Receiver, Sender};
use traits::{AppUpdate, UserIntent};

fn run(rx: Receiver<UserIntent>, tx: Sender<AppUpdate>) {
    loop {
        match rx.recv().unwrap() {
            UserIntent::Exit => {
                tx.send(AppUpdate::Exit).unwrap();
                return
            }
        }
    }
}
