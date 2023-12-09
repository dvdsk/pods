use std::ffi::OsStr;
use std::io::SeekFrom;
use std::num::NonZeroUsize;
use std::path::Path;

use rangemap::RangeSet;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, BufWriter};

use self::progress::Progress;

use super::capacity::Capacity;
use super::{range_watch, SeekInProgress};

mod progress;

#[derive(Debug)]
pub(crate) struct Disk {
    pub(super) capacity: Capacity,
    writer_pos: u64,
    writer: BufWriter<File>,
    reader_pos: u64,
    reader: BufReader<File>,
    progress: Progress,
}

impl Disk {
    pub(super) fn new(
        path: &Path,
        capacity: Capacity,
        _range_tx: range_watch::Sender,
    ) -> Result<Self, ()> {
        if path.extension() == Some(OsStr::new("progress")) {
            todo!("Error invalid path")
        }

        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        let writer = BufWriter::new(fs::File::from_std(file));

        let file = std::fs::OpenOptions::new()
            .read(true)
            .truncate(false)
            .open(&path)
            .unwrap();
        let reader = BufReader::new(fs::File::from_std(file));
        let progress = Progress::new(&path, false).unwrap();

        Ok(Self {
            capacity,
            writer,
            writer_pos: 0,
            reader,
            reader_pos: 0,
            progress,
        })
    }

    pub(super) async fn write_at(
        &mut self,
        buf: &[u8],
        pos: u64,
    ) -> Result<NonZeroUsize, SeekInProgress> {
        if pos != self.writer_pos {
            self.writer.seek(SeekFrom::Start(pos)).await.unwrap();
            self.progress.finish_section(self.writer_pos).unwrap();
            self.writer_pos = pos;
        }
        let written = self.writer.write(buf).await.unwrap();
        Ok(NonZeroUsize::new(written).expect("File should always accept more bytes"))
    }

    pub(super) async fn read_at(&mut self, buf: &mut [u8], pos: u64) -> usize {
        if pos != self.reader_pos {
            self.reader.seek(SeekFrom::Start(pos)).await.unwrap();
            self.reader_pos = pos;
        }
        let read = self.reader.read(buf).await.unwrap();
        read
    }

    pub(super) fn ranges(&self) -> RangeSet<u64> {
        self.progress.ranges.clone()
    }

    pub(super) fn gapless_from_till(&self, _pos: u64, _last_seek: u64) -> bool {
        todo!()
    }

    pub(super) fn set_range_tx(&mut self, _tx: range_watch::Sender) {
        todo!()
    }

    pub(super) fn last_read_pos(&self) -> u64 {
        todo!()
    }

    pub(super) fn n_supported_ranges(&self) -> usize {
        todo!()
    }

    pub(super) fn set_capacity(&mut self, capacity: Capacity) {
        self.capacity = capacity;
    }

    pub(super) fn into_parts(self) -> (range_watch::Sender, Capacity) {
        todo!()
    }
    pub(super) fn clear_for_seek(&mut self, _to_pos: u64) {
        todo!()
    }
}
