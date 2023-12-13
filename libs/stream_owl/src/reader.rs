use std::io::{self, Read, Seek};
use std::sync::MutexGuard;
use std::time::Duration;

use derivative::Derivative;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tracing::instrument;

use crate::store::{self, Gapless, ReadError, SwitchableStore};

mod prefetch;
use prefetch::Prefetch;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Reader {
    #[derivative(Debug = "ignore")]
    rt: Runtime,
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
            rt: Runtime::new().map_err(CouldNotCreateRuntime)?,
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
    #[instrument(level = "debug", err)]
    fn seek(&mut self, rel_pos: io::SeekFrom) -> io::Result<u64> {
        let just_started = self.store.size().requests_analyzed() < 1;

        let pos = match rel_pos {
            io::SeekFrom::Start(bytes) => bytes,
            io::SeekFrom::Current(bytes) => self.curr_pos + bytes as u64,
            io::SeekFrom::End(bytes) => {
                let size = match self.store.size().known() {
                    Some(size) => size,
                    None if just_started => self
                        .store
                        .size()
                        .wait_for_known(&mut self.rt, Duration::from_secs(1))
                        .map_err(|_timeout| size_unknown())?,
                    None => Err(size_unknown())?,
                };
                size.saturating_sub(bytes as u64)
            }
        };

        tracing::debug!("Seeking to: {rel_pos:?}, absolute pos: {pos:?}");
        let gapless = self.store.gapless_from_till(self.last_seek, pos);
        if let Gapless::No(mut store) = gapless {
            self.seek_in_stream(pos)?;
            self.last_seek = pos;
            self.prefetch.reset();
            store.writer_jump(pos);
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
            let bytes = match res {
                Ok(bytes) => bytes,
                Err(ReadError::EndOfStream) => return Ok(0),
                // returns out of block_on closure, not function
                Err(ReadError::Store(other)) => return Err(handle_read_error(other).await),
            };

            self.curr_pos += bytes as u64;
            tracing::info!("actual read: {bytes}");

            self.prefetch
                .perform_if_needed(&mut self.store, self.curr_pos, n_read1 + bytes)
                .await?;

            Ok(bytes)
        })?;

        Ok(n_read1 + n_read2)
    }
}

async fn handle_read_error(_error: store::Error) -> io::Error {
    todo!(
        "notify stream/end stream future with error (some sending required?)
          and turn into an appropriate io::Error for the reader"
    )
}
