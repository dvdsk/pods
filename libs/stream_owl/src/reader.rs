use std::io::{self, Read, Seek};

use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct Reader {
    pub(crate) prefetch: usize,
    pub(crate) seek_tx: mpsc::Sender<u64>,
}

impl Reader {
    pub fn set_prefetch(bytes: usize) {
        todo!()
    }
}

impl Seek for Reader {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        todo!()
    }
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}
