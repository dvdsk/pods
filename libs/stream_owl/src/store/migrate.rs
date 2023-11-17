use std::collections::TryReserveError;
use std::ops::Range;
use std::sync::Arc;

use futures::FutureExt;
use futures_concurrency::future::Race;
use rangemap::set::RangeSet;
use tokio::sync::{oneshot, Mutex};

use super::disk::Disk;
use super::mem::Memory;
use super::{watch, Store, StoreVariant, SwitchableStore};

mod range_list;

impl SwitchableStore {
    pub(crate) async fn to_mem(&self) -> Option<MigrationHandle> {
        if let StoreVariant::Mem = self.variant().await {
            return None;
        }

        let (handle, tx) = MigrationHandle::new();

        // is swapped out before migration finishes
        let (placeholder, _) = watch::channel();
        let mem = match Memory::new(self.capacity.clone(), placeholder) {
            Err(e) => {
                tx.send(Err(MigrationError::MemAllocation(e)))
                    .expect("cant have dropped rx");
                return Some(handle);
            }
            Ok(mem) => mem,
        };
        let migration = migrate(self.curr_store.clone(), Store::Mem(mem), tx);

        tokio::spawn(migration);
        Some(handle)
    }

    pub(crate) async fn to_disk(&self, path: &std::path::Path) -> Option<MigrationHandle> {
        if let StoreVariant::Disk = self.variant().await {
            return None;
        }

        let (handle, tx) = MigrationHandle::new();

        // is swapped out before migration finishes
        let (placeholder, _) = watch::channel();
        let disk = match Disk::new(path, self.capacity.clone(), placeholder) {
            Err(e) => {
                tx.send(Err(MigrationError::DiskCreation(e)))
                    .expect("cant have dropped rx");
                return Some(handle);
            }
            Ok(disk) => disk,
        };
        let migration = migrate(self.curr_store.clone(), Store::Disk(disk), tx);
        tokio::spawn(migration);
        Some(handle)
    }
}

#[derive(Debug)]
pub enum MigrationError {
    DiskCreation(()),
    MemAllocation(TryReserveError),
}

pub struct MigrationHandle(oneshot::Receiver<Result<(), MigrationError>>);

impl MigrationHandle {
    fn new() -> (Self, oneshot::Sender<Result<(), MigrationError>>) {
        let (tx, rx) = oneshot::channel();
        (Self(rx), tx)
    }
}

async fn migrate(
    curr: Arc<Mutex<Store>>,
    mut target: Store,
    mut tx: oneshot::Sender<Result<(), MigrationError>>,
) {
    enum Res {
        Cancelled,
        PreMigration(Result<(), MigrationError>),
    }

    let pre_migration = pre_migrate_to_disk(&curr, &mut target).map(Res::PreMigration);
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

    let mut curr = curr.lock().await;
    let res = finish_migration(&mut curr, &mut target).await;
    if res.is_err() {
        // error is irrelevent if migration is canceld
        let _ = tx.send(res);
    } else {
        let target_ref = &mut target;
        std::mem::swap(&mut *curr, target_ref);
        let old = target;
        curr.set_range_tx(old.into_range_tx());
        curr.capacity_handle().set_total(None);
    }
}

async fn pre_migrate_to_disk(
    curr: &Mutex<Store>,
    target: &mut Store,
) -> Result<(), MigrationError> {
    let mut buf = Vec::with_capacity(4096);
    // TODO handle target that only support one range
    let mut on_target = RangeSet::new();
    loop {
        let src = curr.lock().await;
        let needed_from_src = range_list::needed_ranges(&src, target);
        let needed_form_src = range_list::correct_for_capacity(needed_from_src, target);

        let Some(missing_on_disk) = missing(&on_target, &needed_form_src) else {
            return Ok(());
        };
        let len = missing_on_disk.start - missing_on_disk.end;
        let len = len.min(4096);
        buf.resize(len as usize, 0u8);
        src.read_blocking_at(&mut buf, missing_on_disk.start);
        drop(src);

        target.write_at(&buf, missing_on_disk.start).await;
        on_target.insert(missing_on_disk);
    }
}


async fn finish_migration(curr: &mut Store, target: &mut Store) -> Result<(), MigrationError> {
    let mut buf = Vec::with_capacity(4096);
    let mut on_disk = RangeSet::new();
    loop {
        let in_mem = curr.ranges();
        let Some(missing_on_disk) = missing(&on_disk, &in_mem) else {
            return Ok(());
        };
        let len = missing_on_disk.start - missing_on_disk.end;
        let len = len.min(4096);
        buf.resize(len as usize, 0u8);
        curr.read_blocking_at(&mut buf, missing_on_disk.start);

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

