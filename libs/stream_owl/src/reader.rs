use std::io::{self, Read, Seek};
use std::sync::MutexGuard;
use std::time::{Duration, Instant};

use derivative::Derivative;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tracing::instrument;

use crate::store::{ReadResult, SwitchableStore};

mod prefetch;
use prefetch::Prefetch;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Reader {
    rt: Runtime,
    prefetch: Prefetch,
    #[derivative(Debug = "ignore")]
    seek_tx: mpsc::Sender<u64>,
    last_seek: u64,
    store: SwitchableStore,
    created: Instant,
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
        created: Instant,
    ) -> Result<Self, CouldNotCreateRuntime> {
        Ok(Self {
            rt: Runtime::new().map_err(CouldNotCreateRuntime)?,
            created,
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
    #[instrument(level = "debug", ret, err)]
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let elapsed = self.created.elapsed();
        let just_started = elapsed < Duration::from_secs(1);

        let pos = match pos {
            io::SeekFrom::Start(bytes) => bytes,
            io::SeekFrom::Current(bytes) => self.curr_pos + bytes as u64,
            io::SeekFrom::End(bytes) => {
                let size = match self.store.size().known() {
                    Some(size) => size,
                    None if just_started => self
                        .store
                        .size()
                        .wait_for_known(&mut self.rt, Duration::from_secs(1) - elapsed)
                        .map_err(|_timeout| size_unknown())?,
                    None => Err(size_unknown())?,
                };
                size.saturating_sub(bytes as u64)
            }
        };

        if !self.store.gapless_from_till(self.last_seek, pos) {
            self.seek_in_stream(pos)?;
            self.last_seek = pos;
            self.prefetch.reset();
        }

        self.curr_pos = pos;
        Ok(pos)
    }
}

impl Read for Reader {
    // A read of zero will only happen at the end of file
    #[instrument(level = "trace", skip(buf), ret)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n_read1 = self.prefetch.read_from_prefetched(buf, self.curr_pos);
        self.curr_pos += n_read1 as u64;

        let n_read2 = self.rt.block_on(async {
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
