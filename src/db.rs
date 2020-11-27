use eyre::{Result, WrapErr};
use sled;
use std::path::PathBuf;

pub fn open() -> Result<sled::Db> {
    let path = PathBuf::from("database");
    let config = sled::Config::default()
        .path(&path)
        .cache_capacity(10_000_000) //10mb
        .flush_every_ms(Some(1000));
    let db = config
        .open()
        .wrap_err_with(|| format!("Could not open database on {:?}", path))?;
    Ok(db)
}
