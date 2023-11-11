use std::ops::Range;
use std::sync::Arc;

use futures::FutureExt;
use futures_concurrency::future::Race;
use rangemap::set::RangeSet;
use tokio::sync::{oneshot, Mutex};

use super::disk::Disk;
use super::mem::Memory;
use super::{InnerSwitchableStore, StoreVariant, SwitchableStore};

impl SwitchableStore {
    async fn current(&self) -> StoreVariant {
        match *self.curr.lock().await {
            InnerSwitchableStore::Disk(_) => StoreVariant::Disk,
            InnerSwitchableStore::Mem(_) => StoreVariant::Mem,
        }
    }

    pub(crate) async fn to_mem(&self) -> Option<MigrationHandle> {
        if let StoreVariant::Mem = self.current().await {
            return None;
        }

        let (handle, tx) = MigrationHandle::new();

        let mem = match Memory::new() {
            Err(e) => {
                tx.send(Err(MigrationError::MemAllocation(e)))
                    .expect("cant have dropped rx");
                return Some(handle);
            }
            Ok(mem) => mem,
        };
        let migration = migrate(self.curr.clone(), InnerSwitchableStore::Mem(mem), tx);

        tokio::spawn(migration);
        Some(handle)
    }

    pub(crate) async fn to_disk(&self, path: &std::path::Path) -> Option<MigrationHandle> {
        if let StoreVariant::Disk = self.current().await {
            return None;
        }

        let (handle, tx) = MigrationHandle::new();
        let disk = match Disk::new(path) {
            Err(e) => {
                tx.send(Err(MigrationError::DiskCreation(e)))
                    .expect("cant have dropped rx");
                return Some(handle);
            }
            Ok(disk) => disk,
        };
        let migration = migrate(self.curr.clone(), InnerSwitchableStore::Disk(disk), tx);
        tokio::spawn(migration);
        Some(handle)
    }
}

#[derive(Debug)]
pub enum MigrationError {
    DiskCreation(()),
    MemAllocation(()),
}

pub struct MigrationHandle(oneshot::Receiver<Result<(), MigrationError>>);

impl MigrationHandle {
    fn new() -> (Self, oneshot::Sender<Result<(), MigrationError>>) {
        let (tx, rx) = oneshot::channel();
        (Self(rx), tx)
    }
}

async fn migrate(
    src: Arc<Mutex<InnerSwitchableStore>>,
    mut target: InnerSwitchableStore,
    mut tx: oneshot::Sender<Result<(), MigrationError>>,
) {
    enum Res {
        Cancelled,
        PreMigration(Result<(), MigrationError>),
    }

    let pre_migration = pre_migrate_to_disk(&src, &mut target).map(Res::PreMigration);
    let cancelled = tx.closed().map(|_| Res::Cancelled);
    match (pre_migration, cancelled).race().await {
        Res::Cancelled => return,
        Res::PreMigration(Ok(_)) => (),
        Res::PreMigration(res @ Err(_)) => {
            // error is irrelavent if migration is canceld
            let _ = tx.send(res);
            return;
        }
    }

    let mut src = src.lock().await;
    let res = finish_migration(&mut *src, &mut target).await;
    if res.is_err() {
        // error is irrelavent if migration is canceld
        let _ = tx.send(res);
    } else {
        let src = &mut *src;
        *src = target;
    }
}

async fn pre_migrate_to_disk(
    src: &Mutex<InnerSwitchableStore>,
    target: &mut InnerSwitchableStore,
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
        mem.read_blocking_at(&mut buf, missing_on_disk.start);
        drop(mem);

        target.write_at(&buf, missing_on_disk.start).await;
        on_disk.insert(missing_on_disk);
    }
}

async fn finish_migration(
    src: &mut InnerSwitchableStore,
    target: &mut InnerSwitchableStore,
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
        src.read_blocking_at(&mut buf, missing_on_disk.start);

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
