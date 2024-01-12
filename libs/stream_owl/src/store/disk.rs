use std::ffi::OsStr;
use std::io::SeekFrom;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use derivative::Derivative;
use rangemap::RangeSet;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, BufWriter};
use tracing::instrument;

use self::progress::Progress;

use super::capacity::Capacity;
use super::range_watch;

mod progress;

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Disk {
    // TODO, refactor: this is just kept here for the next store,
    // should take it out and keep it somewhere else
    #[derivative(Debug = "ignore")]
    pub(super) capacity: Capacity,
    writer_pos: u64,
    #[derivative(Debug = "ignore")]
    writer: BufWriter<File>,
    reader_pos: u64,
    #[derivative(Debug = "ignore")]
    reader: BufReader<File>,
    progress: Progress,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not open file to write stream to as writable")]
    OpenForWriting(std::io::Error),
    #[error("Could not open file to write stream to as readable")]
    OpenForReading(std::io::Error),
    #[error("Path may not end with .progress")]
    InvalidPath,
    #[error("Could not write downloaded data to disk")]
    WritingData(std::io::Error),
    #[error("Could not read downloaded data from disk")]
    ReadingData(std::io::Error),
    #[error("Could not load progress from disk or prepare for storing it")]
    OpeningProgress(progress::Error),
    #[error("Could not store download/stream progress to disk")]
    UpdatingProgress(progress::Error),
    #[error("Could not seek while preparing write")]
    SeekForWriting(std::io::Error),
    #[error("Could not seek while preparing read")]
    SeekForReading(std::io::Error),
    #[error("Could not flush data to disk")]
    FlushingData(std::io::Error),
}

impl Disk {
    pub(super) async fn new(
        path: PathBuf,
        capacity: Capacity,
        range_tx: range_watch::Sender,
    ) -> Result<Self, Error> {
        if path.extension() == Some(OsStr::new("progress")) {
            return Err(Error::InvalidPath);
        }

        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&path)
            .await
            .map_err(Error::OpenForWriting)?;
        let writer = BufWriter::new(file);

        let file = fs::OpenOptions::new()
            .read(true)
            .truncate(false)
            .open(&path)
            .await
            .map_err(Error::OpenForReading)?;
        let reader = BufReader::new(file);
        let progress = Progress::new(path, range_tx, 0)
            .await
            .map_err(Error::OpeningProgress)?;

        Ok(Self {
            capacity,
            writer,
            writer_pos: 0,
            reader,
            reader_pos: 0,
            progress,
        })
    }

    async fn writer_seek(&mut self, to: u64) -> Result<(), Error> {
        self.writer
            .seek(SeekFrom::Start(to))
            .await
            .map_err(Error::SeekForWriting)?;
        self.progress
            .finish_section(self.writer_pos, to)
            .await
            .unwrap();
        self.writer_pos = to;
        Ok(())
    }

    #[instrument(level="trace", skip(buf), fields(buf_len = buf.len()), ret)]
    pub(super) async fn write_at(&mut self, buf: &[u8], pos: u64) -> Result<NonZeroUsize, Error> {
        if pos != self.writer_pos {
            self.writer_seek(pos).await?;
        }
        let written = self.writer.write(buf).await.map_err(Error::WritingData)?;
        // OPT: should only flush once read needs it. We could even
        // have a small memory cache that remembers the last unflushed read
        // that way we can lazily flush.
        self.writer.flush().await.map_err(Error::FlushingData)?;
        self.writer_pos += written as u64;
        self.progress
            .update(self.writer_pos)
            .await
            .map_err(Error::UpdatingProgress)?;
        Ok(NonZeroUsize::new(written).expect("File should always accept more bytes"))
    }

    pub(super) async fn read_at(&mut self, buf: &mut [u8], pos: u64) -> Result<usize, Error> {
        if pos != self.reader_pos {
            self.reader
                .seek(SeekFrom::Start(pos))
                .await
                .map_err(Error::SeekForReading)?;
            self.reader_pos = pos;
        }
        self.reader.read(buf).await.map_err(Error::ReadingData)
    }

    pub(super) fn ranges(&self) -> RangeSet<u64> {
        self.progress.ranges.clone()
    }

    pub(super) fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool {
        self.progress
            .ranges
            .gaps(&(pos..last_seek))
            .next()
            .is_none()
    }

    pub(super) fn set_range_tx(&mut self, tx: range_watch::Sender) {
        self.progress.range_tx = tx
    }

    pub(super) fn last_read_pos(&self) -> u64 {
        self.reader_pos
    }

    pub(super) fn n_supported_ranges(&self) -> usize {
        usize::MAX
    }

    pub(super) fn set_capacity(&mut self, capacity: Capacity) {
        self.capacity = capacity;
    }

    pub(super) fn into_parts(self) -> (range_watch::Sender, Capacity) {
        let Self {
            capacity,
            progress: Progress { range_tx, .. },
            ..
        } = self;
        (range_tx, capacity)
    }

    // OPT: see if we can use this to optimize write at
    // (get rid of the seek check)
    pub(super) fn writer_jump(&mut self, _to_pos: u64) {}
}
