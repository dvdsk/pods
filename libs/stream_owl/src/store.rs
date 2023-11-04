use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::AsyncWrite;
use tokio::sync::Mutex;

mod ringbuffer;

pub(crate) trait StreamStore {
    fn size(&self) -> Option<u64>;
    /// returns number of bytes read into buf
    fn read(&self, buf: &mut [u8], curr_pos: u64) -> usize;
    fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool;
}

#[derive(Debug)]
struct Disk;
#[derive(Debug)]
struct Memory;

impl Memory {
    fn from(disk: &mut Disk, pos: u64) -> Result<Self, ()> {
        todo!()
    }
}

impl Disk {
    fn from(memory: &mut Memory, path: &Path) -> Result<Self, ()> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SwitchableStore {
    curr: Arc<Mutex<Store>>,
    prev: Option<Arc<Mutex<Store>>>,
    read_pos: Arc<AtomicU64>,
}

#[derive(Debug)]
enum Store {
    Disk(Disk),
    Memory(Memory),
}

impl SwitchableStore {
    pub(crate) fn new_disk_backed(path: &Path) -> Self {
        Self {
            curr: Arc::new(Mutex::new(Store::Disk(Disk))),
            prev: None,
            read_pos: Arc::new(AtomicU64::new(0u64)),
        }
    }

    pub(crate) fn new_mem_backed() -> Self {
        Self {
            curr: Arc::new(Mutex::new(Store::Memory(Memory))),
            prev: None,
            read_pos: Arc::new(AtomicU64::new(0u64)),
        }
    }

    // TODO opt: two phase switch with background task
    // moving most data first
    pub(crate) fn swith_to_mem_backed(&mut self) -> Result<(), ()> {
        let mut curr = self.curr.blocking_lock();
        let Store::Disk(ref mut disk) = *curr else {
            return Ok(());
        };
        let new = Memory::from(disk, self.read_pos.load(Ordering::Relaxed))?;
        *curr = Store::Memory(new);
        Ok(())
    }

    pub(crate) fn swith_to_disk_backed(&mut self, path: &Path) -> Result<(), ()> {
        let mut curr = self.curr.blocking_lock();
        let Store::Memory(ref mut memory) = *curr else {
            return Ok(());
        };
        let new = Disk::from(memory, path)?;
        *curr = Store::Disk(new);
        Ok(())
    }
}

impl StreamStore for SwitchableStore {
    fn size(&self) -> Option<u64> {
        todo!()
    }

    /// returns number of bytes read into buf
    fn read(&self, buf: &mut [u8], curr_pos: u64) -> usize {
        todo!()
    }

    fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool {
        todo!()
    }
}

impl AsyncWrite for SwitchableStore {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        todo!()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        todo!()
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        todo!()
    }
}
