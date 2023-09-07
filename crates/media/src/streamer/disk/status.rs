use std::ffi::OsStr;
use std::path::Path;

/// Persistent status for a stream to disk
pub(crate) struct Status {}

impl Status {
    fn from_file(path: &Path) -> Status {
        todo!()
    }
}

pub(crate) fn load() -> Vec<Status> {
    std::fs::read_dir(".")
        .unwrap()
        .map(Result::unwrap)
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some(OsStr::new("test")))
        .map(|path| Status::from_file(&path))
        .collect()
}
