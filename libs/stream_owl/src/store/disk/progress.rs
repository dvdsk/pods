use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::mem;
use std::ops::Range;
use std::path::{Path, PathBuf};

use rangemap::RangeSet;

#[derive(Debug)]
pub(super) struct Progress {
    file: File,
    pub ranges: RangeSet<u64>,
}

// a "serialized form" of the RangeSet that can not
// get corrupted by partial writes if the program
// abrubtly crashes
//
// format, binary: <Start of downloaded section><End of downloaded section>
// both as u64 little endian. Therefore each section is 16 bytes long
const SECTION_LENGTH: usize = 2 * mem::size_of::<u64>();

impl Progress {
    pub(super) fn new(file_path: &Path, restart: bool) -> io::Result<Progress> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(restart)
            .open(progress_path(file_path))?;

        if restart {
            todo!("init file")
        }

        let ranges = downloaded(&mut file).unwrap();

        Ok(Progress { file, ranges })
    }

    pub(super) fn finish_section(&self, _end: u64) -> io::Result<()> {
        todo!()
    }
}

fn progress_path(file_path: &Path) -> PathBuf {
    file_path.join(".progress")
}

fn downloaded(file: &mut File) -> io::Result<RangeSet<u64>> {
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let ranges = buf
        .chunks_exact(SECTION_LENGTH)
        .map(|section| section.split_at(SECTION_LENGTH / 2))
        .map(|(a, b)| {
            (
                u64::from_le_bytes(a.try_into().unwrap()),
                u64::from_le_bytes(b.try_into().unwrap()),
            )
        })
        .map(|(start, end)| Range { start, end })
        .collect();
    Ok(ranges)
}
