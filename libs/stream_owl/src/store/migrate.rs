use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use futures::FutureExt;
use futures_concurrency::future::Race;
use rangemap::set::RangeSet;
use tokio::sync::{oneshot, Mutex};

use super::disk::Disk;
use super::limited_mem::{Memory, self};
use super::{capacity, range_watch, CapacityBounds, Store, StoreVariant, SwitchableStore};
use super::{disk, Error as StoreError};
use crate::store;

mod range_list;

impl SwitchableStore {
    pub(crate) async fn to_mem(&self) -> Option<MigrationHandle> {
        if let StoreVariant::MemLimited = self.variant().await {
            return None;
        }

        let (handle, tx) = MigrationHandle::new();

        // is swapped out before migration finishes
        let (watch_placeholder, _) = range_watch::channel();
        let (_, capacity_placeholder) = capacity::new(CapacityBounds::Unlimited);
        let mem = match Memory::new(capacity_placeholder, watch_placeholder) {
            Err(e) => {
                tx.send(Err(MigrationError::MemLimited(e)))
                    .expect("cant have dropped rx");
                return Some(handle);
            }
            Ok(mem) => mem,
        };
        let migration = migrate(self.curr_store.clone(), Store::MemLimited(mem), tx);

        tokio::spawn(migration);
        Some(handle)
    }

    pub(crate) async fn to_disk(&self, path: PathBuf) -> Option<MigrationHandle> {
        if let StoreVariant::Disk = self.variant().await {
            return None;
        }

        let (handle, tx) = MigrationHandle::new();

        // is swapped out before migration finishes
        let (watch_placeholder, _) = range_watch::channel();
        let (_, capacity_placeholder) = capacity::new(CapacityBounds::Unlimited);
        let disk = match Disk::new(path, capacity_placeholder, watch_placeholder).await {
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

#[derive(thiserror::Error, Debug)]
pub enum MigrationError {
    #[error("todo")]
    DiskCreation(disk::Error),
    #[error("todo")]
    MemLimited(limited_mem::Error),
    #[error("todo")]
    WritingToDisk(#[from] StoreError),
    #[error("todo")]
    PreMigrateRead(store::Error),
    #[error("todo")]
    PreMigrateWrite(store::Error),
    #[error("todo")]
    MigrateRead(store::Error),
    #[error("todo")]
    MigrateWrite(store::Error),
}

pub struct MigrationHandle(oneshot::Receiver<Result<(), MigrationError>>);

impl MigrationHandle {
    fn new() -> (Self, oneshot::Sender<Result<(), MigrationError>>) {
        let (tx, rx) = oneshot::channel();
        (Self(rx), tx)
    }

    pub async fn done(self) -> Result<(), MigrationError> {
        self.0
            .await
            .expect("Migration should not crash, therefore never drop the transmit handle")
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

    let pre_migration = pre_migrate(&curr, &mut target).map(Res::PreMigration);
    let cancelled = tx.closed().map(|_| Res::Cancelled);
    match (pre_migration, cancelled).race().await {
        Res::Cancelled => return,
        Res::PreMigration(Ok(_)) => (),
        Res::PreMigration(res @ Err(_)) => {
            // error is irrelevant if migration is canceld
            let _ = tx.send(res);
            return;
        }
    }

    let mut curr = curr.lock().await;
    let res = finish_migration(&mut curr, &mut target).await;
    if res.is_err() {
        // error is irrelevant if migration is canceld
        let _ = tx.send(res);
    } else {
        let target_ref = &mut target;
        std::mem::swap(&mut *curr, target_ref);
        let old = target;
        let (range_watch, capacity) = old.into_parts();
        curr.set_range_tx(range_watch);
        curr.set_capacity(capacity);
        todo!("set up capacity for the new store! (also look into moving capacity out of the Store enum into a (Non clone) Store struct maybe? That would clean up disk)")
    }
}

async fn pre_migrate(curr: &Mutex<Store>, target: &mut Store) -> Result<(), MigrationError> {
    let mut buf = Vec::with_capacity(4096);
    // TODO handle target that only support one range
    let mut on_target = RangeSet::new();
    loop {
        let mut src = curr.lock().await;
        let needed_from_src = range_list::needed_ranges(&src, target);
        let needed_form_src = range_list::correct_for_capacity(needed_from_src, target);

        let Some(missing_on_disk) = missing(&on_target, &needed_form_src) else {
            return Ok(());
        };
        let len = missing_on_disk.start - missing_on_disk.end;
        let len = len.min(4096);
        buf.resize(len as usize, 0u8);
        src.read_at(&mut buf, missing_on_disk.start)
            .await
            .map_err(MigrationError::PreMigrateRead)?;
        drop(src);

        target
            .write_at(&buf, missing_on_disk.start)
            .await
            .map_err(MigrationError::PreMigrateWrite)?;
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
        curr.read_at(&mut buf, missing_on_disk.start)
            .await
            .map_err(MigrationError::MigrateRead)?;

        target
            .write_at(&buf, missing_on_disk.start)
            .await
            .map_err(MigrationError::MigrateWrite)?;
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
