use std::collections::VecDeque;
use std::io::{self, Read, Seek};
use std::sync::MutexGuard;

use derivative::Derivative;
use tokio::sync::mpsc;
use tracing::{debug, instrument};

use crate::store::SwitchableStore;
use crate::{vecd, vecdeque::VecDequeExt};

#[derive(Derivative, Clone)]
#[derivative(Debug)]
struct Prefetch {
    #[derivative(Debug = "ignore")]
    buf: VecDeque<u8>,
    buf_pos: u64,
    active: bool,
}

impl Prefetch {
    /// active by default, to disable just pass in 0 as amount
    fn new(amount: usize) -> Self {
        Self {
            buf: vecd![0; amount],
            buf_pos: 0,
            active: true,
        }
    }

    /// if needed do some prefetching
    #[instrument(level = "debug", ret)]
    fn perform_if_needed(&mut self, store: &mut SwitchableStore, curr_pos: u64, nread: usize) {
        if !self.active {
            return;
        }

        if nread > self.buf.len() {
            return;
        }

        debug!("prefetching");
        let (_used, free) = self.buf.as_mut_slices();
        store.read_blocking_at(free, curr_pos);
        self.active = false
    }

    #[instrument(level = "trace", ret)]
    fn read_from_prefetched(&mut self, buf: &mut [u8], curr_pos: u64) -> usize {
        let relative_pos = curr_pos - self.buf_pos;
        let n_copied = self.buf.copy_starting_at(relative_pos as usize, buf);
        n_copied
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Reader {
    prefetch: Prefetch,
    #[derivative(Debug = "ignore")]
    seek_tx: mpsc::Sender<u64>,
    last_seek: u64,
    store: SwitchableStore,
    curr_pos: u64,
}

impl Reader {
    pub(crate) fn new(
        _guard: MutexGuard<()>,
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
    // this moves the seek in the stream
    #[instrument(level = "debug", ret)]
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let pos = match pos {
            io::SeekFrom::Start(bytes) => bytes,
            io::SeekFrom::End(bytes) => self
                .store
                .size()
                .ok_or(size_unknown())?
                .saturating_sub(bytes as u64),
            io::SeekFrom::Current(bytes) => self.curr_pos + bytes as u64,
        };

        if !self.store.gapless_from_till(self.last_seek, pos) {
            self.seek_in_stream(pos)?;
            self.last_seek = pos;
            self.prefetch.active = true;
        }

        Ok(pos)
    }
}

impl Read for Reader {
    // A read of zero will only happen at the end of file
    #[instrument(level = "trace", skip(buf), ret)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n_read1 = self.prefetch.read_from_prefetched(buf, self.curr_pos);
        self.curr_pos += n_read1 as u64;

        let n_read2 = self
            .store
            .read_blocking_at(&mut buf[n_read1..], self.curr_pos);
        self.curr_pos += n_read2 as u64;

        self.prefetch
            .perform_if_needed(&mut self.store, self.curr_pos, n_read1 + n_read2);
        Ok(n_read1 + n_read2)
    }
}
