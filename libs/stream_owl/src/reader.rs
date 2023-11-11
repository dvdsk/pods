use std::io::{self, Read, Seek};
use std::sync::MutexGuard;

use tokio::sync::mpsc;

use crate::store::{SwitchableStore, InnerSwitchableStore};

#[derive(Debug, Clone)]
struct Prefetch {
    buf: Vec<u8>,
    active: bool,
}

impl Prefetch {
    /// active by default, to disable just pase in 0 as amount
    fn new(amount: usize) -> Self {
        Self {
            buf: vec![0u8; amount],
            active: true,
        }
    }

    /// if needed do some prefetching
    fn perform_if_needed(&mut self, store: &InnerSwitchableStore, curr_pos: u64) {
        if !self.active {
            return;
        }

        store.read_blocking_at(&mut self.buf, curr_pos);
        self.active = false
    }
}

#[derive(Debug, Clone)]
pub struct Reader {
    prefetch: Prefetch,
    seek_tx: mpsc::Sender<u64>,
    last_seek: u64,
    store: SwitchableStore,
    curr_pos: u64,
}

impl Reader {
    pub(crate) fn new(
        guard: MutexGuard<()>,
        prefetch: usize,
        seek_tx: mpsc::Sender<u64>,
        store: SwitchableStore,
    ) -> Self {
        Self {
            prefetch: Prefetch::new(prefetch),
            seek_tx,
            last_seek: 0,
            store,
            curr_pos: 0,
        }
    }

    fn seek_in_stream(&mut self, pos: u64) -> io::Result<()> {
        self.seek_tx.blocking_send(pos).map_err(stream_ended)
    }
}

fn size_unknown() -> io::Error {
    io::Error::new(
        io::ErrorKind::Other,
        "could not seek from end, as size is unknown",
    )
}

fn stream_ended(_: tokio::sync::mpsc::error::SendError<u64>) -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "stream was ended")
}

impl Seek for Reader {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let store = self.store.blocking_lock();
        let pos = match pos {
            io::SeekFrom::Start(bytes) => bytes,
            io::SeekFrom::End(bytes) => store
                .size()
                .ok_or(size_unknown())?
                .saturating_sub(bytes as u64),
            io::SeekFrom::Current(bytes) => self.curr_pos + bytes as u64,
        };

        if !store.gapless_from_till(self.last_seek, pos) {
            drop(store);
            self.seek_in_stream(pos)?;
            self.last_seek = pos;
            self.prefetch.active = true;
        }

        Ok(pos)
    }
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut store = self.store.blocking_lock();
        let n_read = store.read_blocking_at(buf, self.curr_pos);

        self.prefetch.perform_if_needed(&mut *store, self.curr_pos);
        self.curr_pos += n_read as u64;
        Ok(n_read)
    }
}
