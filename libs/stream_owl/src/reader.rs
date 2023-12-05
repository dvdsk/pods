use std::collections::VecDeque;
use std::io::{self, Read, Seek};
use std::sync::MutexGuard;

use derivative::Derivative;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tracing::{debug, instrument};

use crate::store::{ReadResult, SwitchableStore};
use crate::{vecd, vecdeque::VecDequeExt};

#[derive(Derivative, Clone)]
#[derivative(Debug)]
struct Prefetch {
    #[derivative(Debug = "ignore")]
    buf: VecDeque<u8>,
    /// Position in the stream of the last byte 
    /// in the prefetch buffer
    buf_ends_at: u64,
    active: bool,
}

impl Prefetch {
    /// active by default, to disable just pass in 0 as amount
    fn new(amount: usize) -> Self {
        Self {
            buf: vecd![0; amount],
            buf_ends_at: 0,
            active: true,
        }
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.buf_ends_at = 0;
        self.active = true;
    }

    /// if needed do some prefetching
    #[instrument(level = "debug", ret)]
    async fn perform_if_needed(
        &mut self,
        store: &mut SwitchableStore,
        curr_pos: u64,
        already_read: usize,
    ) {
        if !self.active {
            return;
        }

        if already_read >= self.buf.len() {
            return;
        }

        debug!("prefetching");
        assert_eq!(self.buf_ends_at, 0);
        let mut to_prefetch = self.buf.len() - already_read;
        if let Some(stream_size) = store.size().known() {
            to_prefetch = to_prefetch.min((stream_size - curr_pos) as usize);
        }

        let (a, b) = self.buf.as_mut_slices();
        while self.buf_ends_at as usize <= a.len() {
            let start = self.buf_ends_at as usize;
            let end = start + to_prefetch.min(a.len() - start);
            let free = &mut a[start..end];
            let ReadResult::ReadN(bytes) = store.read_at(free, curr_pos + self.buf_ends_at).await
            else {
                debug!("Prefetch aborted, end of stream reached");
                return; // next call to read_at will signal eof
            };

            self.buf_ends_at += bytes as u64;
            to_prefetch -= bytes;
        }

        while !self.buf_ends_at as usize >= a.len() + b.len() {
            let start = self.buf_ends_at as usize - a.len();
            let end = start + to_prefetch.min(b.len() - start);
            let free = &mut b[start..end];
            let ReadResult::ReadN(bytes) = store.read_at(free, curr_pos + self.buf_ends_at).await
            else {
                debug!("Prefetch aborted, end of stream reached");
                return; // next call to read_at will signal eof
            };

            self.buf_ends_at += bytes as u64;
            to_prefetch -= bytes;
        }
        self.active = false
    }

    #[instrument(level = "trace", ret)]
    fn read_from_prefetched(&mut self, buf: &mut [u8], curr_pos: u64) -> usize {
        let relative_pos = curr_pos + self.buf_ends_at - self.buf.len() as u64;
        let n_copied = self.buf.copy_starting_at(relative_pos as usize, buf);
        n_copied
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Reader {
    runtime: Runtime,
    prefetch: Prefetch,
    #[derivative(Debug = "ignore")]
    seek_tx: mpsc::Sender<u64>,
    last_seek: u64,
    store: SwitchableStore,
    curr_pos: u64,
}

#[derive(Debug)]
pub struct CouldNotCreateRuntime(io::Error);

impl Reader {
    pub(crate) fn new(
        _guard: MutexGuard<()>,
        prefetch: usize,
        seek_tx: mpsc::Sender<u64>,
        store: SwitchableStore,
    ) -> Result<Self, CouldNotCreateRuntime> {
        Ok(Self {
            runtime: Runtime::new().map_err(CouldNotCreateRuntime)?,
            prefetch: Prefetch::new(prefetch),
            seek_tx,
            last_seek: 0,
            store,
            curr_pos: 0,
        })
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
                .known()
                .ok_or(size_unknown())?
                .saturating_sub(bytes as u64),
            io::SeekFrom::Current(bytes) => self.curr_pos + bytes as u64,
        };

        if !self.store.gapless_from_till(self.last_seek, pos) {
            self.seek_in_stream(pos)?;
            self.last_seek = pos;
            self.prefetch.reset();
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

        let n_read2 = self.runtime.block_on(async {
            let res = self.store.read_at(&mut buf[n_read1..], self.curr_pos).await;
            let ReadResult::ReadN(bytes) = res else {
                return 0; // returns out of block_on closure, not function
            };
            self.curr_pos += bytes as u64;
            tracing::info!("actual read: {bytes}");

            self.prefetch
                .perform_if_needed(&mut self.store, self.curr_pos, n_read1 + bytes)
                .await;
            bytes
        });
        Ok(n_read1 + n_read2)
    }
}
