use std::future::{self, Future};

use futures::{stream, Stream};

/// Interface for reading streaming/downloading data
mod reader;
mod storage;

#[derive(Debug)]
pub struct ManageError;
#[derive(Debug)]
pub struct StreamError;

#[derive(Debug)]
pub struct StreamId(usize);

pub struct Manager;

pub struct StreamHandle;
impl StreamHandle {
    pub fn set_priority(&self, arg: i32) {
        todo!()
    }

    pub fn try_get_reader(&self) -> reader::Reader {
        todo!()
    }
    pub fn get_id(&self) -> StreamId {
        todo!()
    }
}

impl Manager {
    pub fn new() -> (
        Self,
        impl Future<Output = ManageError>,
        impl Stream<Item = (StreamId, StreamError)>,
    ) {
        (Self, future::pending(), stream::pending())
    }

    pub fn new_to_disk(&mut self, url1: &str) -> StreamHandle {
        StreamHandle
    }

    pub fn new_to_mem(&mut self, url2: &str) -> StreamHandle {
        StreamHandle
    }
}
