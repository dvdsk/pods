use std::future::{self, Future};

/// Interface for reading streaming/downloading data
mod reader;
mod storage;

#[derive(Debug)]
pub struct ManageError;
#[derive(Debug)]
pub struct StreamError;

pub struct Manager;

pub struct StreamHandle;
impl StreamHandle {
    pub fn set_priority(&self, arg: i32) {
        todo!()
    }

    pub fn try_get_reader(&self) -> reader::Reader {
        todo!()
    }
}

impl Manager {
    pub fn new() -> (Self, impl Future<Output = ManageError>) {
        (Self, future::pending())
    }

    pub fn add_disk(&mut self, url1: &str) -> (StreamHandle, impl Future<Output = StreamError>) {
        (StreamHandle, future::pending())
    }

    pub fn add_mem(&mut self, url2: &str) -> (StreamHandle, impl Future<Output = StreamError>) {
        (StreamHandle, future::pending())
    }
}
