use crate::db;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const LEAP: u64 = 100;

pub(super) struct Leases {
    podcast_id: AtomicBool,
    episode_id: AtomicBool,
}

// should create only one
impl Leases {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Leases {
            podcast_id: AtomicBool::new(false),
            episode_id: AtomicBool::new(false),
        })
    }
}

macro_rules! impl_idgen {
    ($struct:ident, $field:ident) => {
        pub(crate) struct $struct {
            next_free: Option<u64>,
            leases: Arc<Leases>,
            db: Arc<db::Store>,
        }

        impl $struct {
            pub fn new(db: Arc<db::Store>, leases: Arc<Leases>) -> Self {
                let res = leases.$field.compare_exchange(
                    false,
                    true,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                );
                if res.is_err() {
                    panic!("Can only run one IdGen simultatiously")
                }

                Self {
                    leases,
                    next_free: None,
                    db,
                }
            }

            fn update_in_db(db: &db::Store) -> u64 {
                let id = db.$field().get().unwrap();
                let new_id = id + LEAP;
                db.$field().set(&new_id).unwrap();
                new_id
            }
        }

        impl Drop for $struct {
            fn drop(&mut self) {
                self.leases.$field.store(false, Ordering::Relaxed);
            }
        }

        impl traits::IdGen for $struct {
            fn next(&mut self) -> u64 {
                // init if not init
                if self.next_free.is_none() {
                    self.next_free = Some(Self::update_in_db(&self.db));
                };
                let next_free = self.next_free.as_mut().unwrap();

                if *next_free % LEAP == 0 {
                    *next_free = Self::update_in_db(&self.db);
                }

                let next_id: u64 = *next_free;
                *next_free += 1;
                next_id
            }
        }
    };
}

impl_idgen! {PodcastIdGen, podcast_id}
impl_idgen! {EpisodeIdGen, episode_id}

#[cfg(test)]
mod tests {
    use super::*;
    use traits::IdGen;

    /// note implementation detail! feel free to break
    /// this test in the future if impl changes!
    #[test]
    fn monotonically_increasing() {
        let tempdir = tempfile::tempdir().unwrap();
        let db = Arc::new(db::Store::new(tempdir).unwrap());
        let leases = Leases::new();
        let mut id_gen = PodcastIdGen::new(db, leases);

        let mut prev = 0;
        for _ in 0..2000 {
            let curr = id_gen.next();
            assert!(curr > prev);
            prev = curr;
        }
    }

    #[test]
    fn monotonically_increasing_between_runs() {
        let tempdir = tempfile::tempdir().unwrap();
        let db = Arc::new(db::Store::new(tempdir).unwrap());
        let leases = Leases::new();
        let mut id_gen = PodcastIdGen::new(db.clone(), leases.clone());

        let mut prev = 0;
        for _ in 0..2000 {
            let curr = id_gen.next();
            assert!(curr > prev);
            prev = curr;
        }

        std::mem::drop(id_gen); // here for readability
        let mut id_gen = PodcastIdGen::new(db, leases);
        for _ in 0..10 {
            let curr = id_gen.next();
            assert!(curr > prev);
            prev = curr;
        }
    }
}
