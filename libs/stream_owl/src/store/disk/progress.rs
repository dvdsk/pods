use derivative::Derivative;
use std::ops::Range;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{io, mem};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use rangemap::RangeSet;
use tracing::{debug, instrument};

use crate::store::range_watch;

// TODO make append only + regular cleanup use sys atomic move file

// a "serialized form" of the RangeSet that can not
// get corrupted by partial writes if the program
// abrubtly crashes
//
// format, binary: <Start of downloaded section><End of downloaded section>
// both as u64 little endian. Therefore each section is 16 bytes long
const SECTION_LEN: usize = 2 * mem::size_of::<u64>();

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not create or open file storing download progress")]
    OpeningFile(io::Error),
    #[error("Could not read list of already downloaded parts")]
    ReadingProgress(io::Error),
    #[error("Could not write progress to file")]
    AppendingSection(io::Error),
    #[error("Could not flush written progress to disk")]
    Flushing(io::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Derivative)]
#[derivative(Debug)]
pub(super) struct Progress {
    /// The cursor is where the next append mark should be written
    #[derivative(Debug = "ignore")]
    file: File,
    next_record_start: u64,
    #[derivative(Debug(format_with = "fmt_last_flush"))]
    last_flush: Instant,
    pub(super) ranges: RangeSet<u64>,
    #[derivative(Debug = "ignore")]
    pub(super) range_tx: range_watch::Sender,
}

fn fmt_last_flush(
    instant: &Instant,
    fmt: &mut std::fmt::Formatter,
) -> std::result::Result<(), std::fmt::Error> {
    fmt.write_fmt(format_args!("{}ms ago", instant.elapsed().as_millis()))
}

#[derive(Debug)]
pub(super) enum FlushNeeded {
    Yes,
    No,
}

impl Progress {
    #[instrument(level = "debug", skip(range_tx))]
    pub(super) async fn new(
        file_path: PathBuf,
        range_tx: range_watch::Sender,
        start_pos: u64,
    ) -> Result<Progress> {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(progress_path(file_path))
            .await
            .map_err(Error::OpeningFile)?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .await
            .map_err(Error::ReadingProgress)?;

        let ranges = ranges_from_file_bytes(&buf);
        if !ranges.is_empty() {
            debug!("loaded existing progress from file, sections already downloaded: {ranges:?}");
        }

        for range in ranges.iter().cloned() {
            range_tx.send(range);
        }

        Ok(Progress {
            file,
            ranges,
            last_flush: Instant::now(),
            next_record_start: start_pos,
            range_tx,
        })
    }

    /// Data needs to be already flushed before this is called
    #[instrument(level = "debug", skip(self))]
    pub(super) async fn end_section(&mut self, end: u64, new_starts_at: u64) -> FlushNeeded {
        let res = self.update(end).await;
        self.next_record_start = new_starts_at;
        debug!("ended section");
        res
    }

    #[instrument(level = "trace", skip(self), ret)]
    pub(super) async fn update(&mut self, writer_pos: u64) -> FlushNeeded {
        let new_range = self.next_record_start..writer_pos;
        if new_range.is_empty() {
            return FlushNeeded::No;
        }
        self.ranges.insert(new_range.clone());
        self.range_tx.send(new_range);

        if self.last_flush.elapsed() > Duration::from_millis(500) {
            FlushNeeded::Yes
        } else {
            FlushNeeded::No
        }
    }

    /// Data needs to be already flushed before this is called
    #[instrument(level = "trace", ret, err)]
    pub(super) async fn flush(&mut self) -> Result<()> {
        let start = self.next_record_start;
        if let Some(non_empty_range) = self.ranges.get(&start).cloned() {
            self.record_section(non_empty_range.clone()).await?;
            self.next_record_start = non_empty_range.end;
        }
        Ok(())
    }

    #[instrument(level = "debug", skip(self), ret)]
    async fn record_section(&mut self, Range { start, end }: Range<u64>) -> Result<()> {
        tracing::debug!("recording section: {start}-{end}");
        self.ranges.insert(start..end);

        let mut final_section = [0u8; SECTION_LEN];
        final_section[..8].copy_from_slice(&start.to_le_bytes());
        final_section[8..].copy_from_slice(&end.to_le_bytes());
        self.file
            .write_all(&final_section)
            .await
            .map_err(Error::AppendingSection)?;
        self.file.flush().await.map_err(Error::Flushing)?;
        self.last_flush = Instant::now();
        Ok(())
    }
}

#[instrument(level = "trace", ret)]
fn progress_path(mut file_path: PathBuf) -> PathBuf {
    file_path.as_mut_os_string().push(".progress");
    file_path
}

#[instrument(level = "debug", skip_all, fields(buf_len= buf.len()))]
fn ranges_from_file_bytes(buf: &[u8]) -> RangeSet<u64> {
    buf.chunks_exact(SECTION_LEN)
        .map(|section| section.split_at(SECTION_LEN / 2))
        .map(|(a, b)| {
            (
                u64::from_le_bytes(a.try_into().unwrap()),
                u64::from_le_bytes(b.try_into().unwrap()),
            )
        })
        .map(|(start, end)| Range { start, end })
        .filter(|range| !range.is_empty())
        .collect()
}
