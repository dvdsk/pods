use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use rangemap::RangeSet;
use tokio::sync::Mutex;

use self::switch::StoreVariant;

mod ringbuffer;

mod disk;
mod mem;
mod switch;

#[derive(Debug, Clone)]
pub(crate) struct SwitchableStore {
    curr: Arc<Mutex<Store>>,
    prev: Arc<Mutex<Option<Store>>>,
    read_pos: Arc<AtomicU64>,
    switch: switch::Handle,
}

#[derive(Debug)]
enum Store {
    Disk(disk::Disk),
    Memory(mem::Memory),
}

impl Store {
    async fn write(&mut self, buf: &[u8]) {
        match self {
            Store::Disk(disk) => disk.write(buf),
            Store::Memory(mem) => mem.write(buf),
        }
    }
    async fn read(&self, buf: &mut [u8], at_pos: u64) -> usize {
        match self {
            Store::Disk(disk) => disk.read(buf, at_pos),
            Store::Memory(mem) => mem.read_at(buf, at_pos),
        }
    }
}

impl SwitchableStore {
    pub(crate) fn new_disk_backed(path: PathBuf) -> Self {
        Self {
            curr: Arc::new(Mutex::new(Store::Disk(disk::Disk))),
            prev: Arc::new(Mutex::new(None)),
            read_pos: Arc::new(AtomicU64::new(0u64)),
        }
    }

    pub(crate) fn new_mem_backed() -> Self {
        Self {
            curr: Arc::new(Mutex::new(Store::Memory(mem::Memory))),
            prev: Arc::new(Mutex::new(None)),
            read_pos: Arc::new(AtomicU64::new(0u64)),
        }
    }

    // TODO need two phase switch with background task
    // moving most data first
    pub(crate) async fn swith_to_mem_backed(&mut self) -> Result<(), ()> {
        self.switch.to_mem();
        Ok(())
    }

    pub(crate) async fn swith_to_disk_backed(&mut self, path: &Path) -> Result<(), ()> {
        self.switch.to_disk(path);
        Ok(())
    }
}

#[async_trait::async_trait]
trait StoreTomato {
    async fn write_at(&self, buf: &[u8], pos: u64);
    async fn read_at(&self, buf: &mut [u8], pos: u64) -> usize;
    fn ranges(&self) -> RangeSet<u64>;
    fn variant(&self) -> StoreVariant;
}

impl SwitchableStore {
    pub(crate) fn size(&self) -> Option<u64> {
        todo!()
    }

    /// returns number of bytes read into buf
    pub(crate) async fn read(&self, buf: &mut [u8], mut curr_pos: u64) -> usize {
        let mut n_read = 0;
        let prev = self.prev.lock().await;
        if let Some(prev) = prev.as_ref() {
            n_read += prev.read(buf, curr_pos).await;
            if n_read >= buf.len() {
                return n_read;
            }
            curr_pos += n_read as u64;
        }

        let curr = self.curr.lock().await;
        n_read += curr.read(buf, curr_pos).await;
        n_read
    }

    pub(crate) fn gapless_from_till(&self, pos: u64, last_seek: u64) -> bool {
        todo!()
    }
}

impl SwitchableStore {
    async fn write(&mut self, buf: &[u8]) {
        let mut curr = self.curr.lock().await;
        curr.write(buf).await;
    }
}
