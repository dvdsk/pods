use std::collections::VecDeque;
use std::ops::Range;

use derivative::Derivative;
use tracing::debug;
use tracing::instrument;

use crate::reader::handle_read_error;
use crate::store::ReadError;
use crate::store::SwitchableStore;
use crate::vecd;
use crate::vecdeque::VecDequeExt;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub(crate) struct Prefetch {
    #[derivative(Debug = "ignore")]
    pub(crate) buf: VecDeque<u8>,
    /// Position in the stream of the first byte
    /// in the prefetch buffer
    pub(crate) in_buffer: Range<u64>,
    pub(crate) active: bool,
}

impl Prefetch {
    /// active by default, to disable just pass in 0 as amount
    pub(crate) fn new(amount: usize) -> Self {
        Self {
            buf: vecd![0; amount],
            in_buffer: 0..0,
            active: true,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.buf.clear();
        self.in_buffer = 0..0;
        self.active = true;
    }

    fn should_run(&self, already_read: usize) -> bool {
        if !self.active {
            return false;
        }
        if already_read >= self.buf.len() {
            return false;
        }
        debug!("prefetching");
        true
    }

    fn bytes_to_prefetch(
        &self,
        already_read: usize,
        curr_pos: u64,
        store: &SwitchableStore,
    ) -> usize {
        let mut to_prefetch = self.buf.len() - already_read;
        if let Some(stream_size) = store.size().known() {
            to_prefetch = to_prefetch.min((stream_size - curr_pos) as usize);
        }
        to_prefetch
    }

    /// if needed do some prefetching
    #[instrument(level = "debug", ret)]
    pub(crate) async fn perform_if_needed(
        &mut self,
        store: &mut SwitchableStore,
        reader_pos: u64,
        already_read: usize,
    ) -> Result<(), std::io::Error> {
        if !self.should_run(already_read) {
            return Ok(());
        }
        self.active = false;

        let mut prefetching = Prefetching {
            pos: reader_pos,
            still_needed: self.bytes_to_prefetch(already_read, reader_pos, store),
            store,
        };

        let (a, b) = self.buf.as_mut_slices();
        match prefetching.read_into_first(a).await {
            Err(ReadError::EndOfStream) => debug!("Prefetch ended_early, end of stream reached"),
            Err(ReadError::Store(e)) => return Err(handle_read_error(e).await),
            Ok(_) => match prefetching.read_into_second(b).await {
                Ok(_) => todo!(),
                Err(ReadError::EndOfStream) => {
                    debug!("Prefetch ended_early, end of stream reached")
                }
                Err(ReadError::Store(e)) => return Err(handle_read_error(e).await),
            },
        }

        self.in_buffer = prefetching.finish(reader_pos);
        Ok(())
    }

    #[instrument(level = "trace", skip(buf), ret)]
    pub(crate) fn read_from_prefetched(&mut self, buf: &mut [u8], curr_pos: u64) -> usize {
        if !self.in_buffer.contains(&curr_pos) {
            return 0;
        }

        let offset = curr_pos - self.in_buffer.start;
        let n_copied = self.buf.copy_starting_at(offset as usize, buf);
        n_copied
    }
}

struct Prefetching<'a> {
    pos: u64,
    still_needed: usize,
    store: &'a mut SwitchableStore,
}

impl<'a> Prefetching<'a> {
    fn account_for_bytes_read(&mut self, bytes: usize) {
        self.still_needed -= bytes;
        self.pos += bytes as u64;
    }

    async fn read_into_first(&mut self, a: &mut [u8]) -> Result<(), ReadError> {
        let n_read = 0;
        while n_read <= a.len() && self.still_needed > 0 {
            let start = n_read;
            let space_left = a.len() - n_read;
            let end = start + self.still_needed.min(space_left);
            let free = &mut a[start..end];
            let bytes = self.store.read_at(free, self.pos).await?;
            self.account_for_bytes_read(bytes);
        }
        Ok(())
    }

    async fn read_into_second(&mut self, b: &mut [u8]) -> Result<(), ReadError> {
        let n_read = 0;
        while n_read <= b.len() && self.still_needed > 0 {
            let start = n_read;
            // buffer is set up such that there is always enough
            // space for the entire prefetch
            let end = start + self.still_needed;
            let free = &mut b[start..end];
            let bytes = self.store.read_at(free, self.pos).await?;
            self.account_for_bytes_read(bytes);
        }
        Ok(())
    }

    fn finish(&self, start: u64) -> Range<u64> {
        Range {
            start,
            end: self.pos,
        }
    }
}
