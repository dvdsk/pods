use std::ops::Range;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use futures::FutureExt;
use futures_concurrency::future::Race;
use rangemap::set::RangeSet;
use tokio::sync::{oneshot, Mutex};

use super::disk::Disk;
use super::mem::Memory;
use super::StoreTomato;

#[derive(Debug)]
pub(super) enum StoreVariant {
    Disk = 0,
    Mem = 1,
}

#[derive(Debug)]
pub(super) enum Switching {
    No,
    ToDisk,
    ToMem,
}

#[derive(Debug)]
pub(super) struct Handle {
    migrating_to: Option<StoreVariant>,
    curr: Arc<AtomicU8>,
    disk: Arc<Mutex<Option<Disk>>>,
    mem: Arc<Mutex<Option<Memory>>>,
}

impl Handle {
    async fn current(&self) -> StoreVariant {
        match self.curr.load(Ordering::Acquire) {
            0 => StoreVariant::Disk,
            1 => StoreVariant::Mem,
            2.. => unreachable!(),
        }
    }

    pub(crate) async fn to_mem(&self) -> Option<MigrationHandle> {
        if let StoreVariant::Mem = self.current().await {
            return None;
        }

        let (handle, tx) = MigrationHandle::new();
        let migration = migrate(self.disk.clone(), self.mem.clone(), self.curr.clone(), tx);
        tokio::spawn(migration);
        Some(handle)
    }

    pub(crate) async fn to_disk(&self, path: &std::path::Path) -> Option<MigrationHandle> {
        if let StoreVariant::Disk = self.current().await {
            return None;
        }

        let (handle, tx) = MigrationHandle::new();
        let migration = migrate(self.mem.clone(), self.disk.clone(), self.curr.clone(), tx);
        tokio::spawn(migration);
        Some(handle)
    }
}

pub enum MigrationError {}

pub struct MigrationHandle(oneshot::Receiver<Result<(), MigrationError>>);

impl MigrationHandle {
    fn new() -> (Self, oneshot::Sender<Result<(), MigrationError>>) {
        let (tx, rx) = oneshot::channel();
        (Self(rx), tx)
    }
}

async fn migrate(
    src: Arc<Mutex<impl StoreTomato>>,
    target: Arc<Mutex<impl StoreTomato>>,
    current: Arc<AtomicU8>,
    mut tx: oneshot::Sender<Result<(), MigrationError>>,
) {
    let mut target = target.lock().await;
    enum Res {
        Cancelled,
        PreMigration(Result<(), MigrationError>),
    }

    let pre_migration = pre_migrate_to_disk(&src, &mut *target).map(Res::PreMigration);
    let cancelled = tx.closed().map(|_| Res::Cancelled);
    match (pre_migration, cancelled).race().await {
        Res::Cancelled => return,
        Res::PreMigration(Ok(_)) => (),
        Res::PreMigration(res @ Err(_)) => {
            tx.send(res);
            return;
        }
    }

    let mut src = src.lock().await;
    let res = finish_migration(&mut *src, &mut *target).await;
    if res.is_err() {
        tx.send(res);
    } else {
        current.store(target.variant() as u8, Ordering::Release)
    }
}

async fn pre_migrate_to_disk(
    src: &Mutex<impl StoreTomato>,
    target: &mut impl StoreTomato,
) -> Result<(), MigrationError> {
    let mut buf = Vec::with_capacity(4096);
    // nobody needs to read disk while we are migrating to it
    let mut on_disk = RangeSet::new();
    loop {
        let mem = src.lock().await;
        let in_mem = mem.ranges();

        let Some(missing_on_disk) = missing(&on_disk, &in_mem) else {
            return Ok(());
        };
        let len = missing_on_disk.start - missing_on_disk.end;
        let len = len.min(4096);
        buf.resize(len as usize, 0u8);
        mem.read_at(&mut buf, missing_on_disk.start);
        drop(mem);

        target.write_at(&buf, missing_on_disk.start).await;
        on_disk.insert(missing_on_disk);
    }
}

async fn finish_migration(
    src: &mut impl StoreTomato,
    target: &mut impl StoreTomato,
) -> Result<(), MigrationError> {
    let mut buf = Vec::with_capacity(4096);
    let mut on_disk = RangeSet::new();
    loop {
        let in_mem = src.ranges();
        let Some(missing_on_disk) = missing(&on_disk, &in_mem) else {
            return Ok(());
        };
        let len = missing_on_disk.start - missing_on_disk.end;
        let len = len.min(4096);
        buf.resize(len as usize, 0u8);
        src.read_at(&mut buf, missing_on_disk.start);

        target.write_at(&buf, missing_on_disk.start).await;
        on_disk.insert(missing_on_disk);
    }
}

/// return a range that exists in b but not in a
fn missing(a: &RangeSet<u64>, b: &RangeSet<u64>) -> Option<Range<u64>> {
    let mut in_b = b.iter();
    loop {
        let missing_in_a = a.gaps(in_b.next()?).next();
        if missing_in_a.is_some() {
            return missing_in_a;
        }
    }
}
