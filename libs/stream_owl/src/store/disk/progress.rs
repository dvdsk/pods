use derivative::Derivative;
use std::ops::Range;
use std::path::PathBuf;
use std::{io, mem};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use rangemap::RangeSet;
use tracing::instrument;

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
    pub(super) ranges: RangeSet<u64>,
    #[derivative(Debug = "ignore")]
    pub(super) range_tx: range_watch::Sender,
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

        Ok(Progress {
            file,
            ranges: ranges_from_file_bytes(&buf),
            next_record_start: start_pos,
            range_tx,
        })
    }

    #[instrument(level = "debug", ret, err)]
    pub(super) async fn finish_section(&mut self, end: u64, new_starts_at: u64) -> Result<()> {
        let start = self.next_record_start;
        self.record_section(start, end).await?;
        self.next_record_start = new_starts_at;
        Ok(())
    }

    #[instrument(level = "trace", ret, err)]
    pub(super) async fn update(&mut self, writer_pos: u64) -> Result<()> {
        let new_range = self.next_record_start..writer_pos;
        self.range_tx.send(new_range);

        let written_since_recorded = writer_pos - self.next_record_start;
        match written_since_recorded {
            0..=4_999 => Ok(()),
            5_000..=9_999 => self.file.flush().await.map_err(Error::Flushing),
            10_000.. => {
                let start = self.next_record_start;
                self.record_section(start, writer_pos).await
            }
        }
    }

    #[instrument(level="debug", skip(self))]
    async fn record_section(&mut self, start: u64, end: u64) -> Result<()> {
        if start != end {
            self.ranges.insert(start..end);
        }

        let mut final_section = [0u8; SECTION_LEN];
        final_section[..8].copy_from_slice(&start.to_le_bytes());
        final_section[8..].copy_from_slice(&end.to_le_bytes());
        self.file
            .write_all(&final_section)
            .await
            .map_err(Error::AppendingSection)?;
        Ok(())
    }
}

#[instrument(level = "debug", ret)]
fn progress_path(mut file_path: PathBuf) -> PathBuf {
    file_path.as_mut_os_string().push(".progress");
    file_path
}

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
