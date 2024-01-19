use rangemap::set::RangeSet;
use tracing::instrument;

use super::super::Store;

use std::ops::Range;

pub(crate) struct RangeListIterator {
    pub(crate) list: Vec<Range<u64>>,
    pub(crate) dist_from_center: isize,
    pub(crate) mul: isize,
}

impl Iterator for RangeListIterator {
    type Item = Range<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        let center = self.list.len() / 2;
        let idx = center as isize + self.mul * self.dist_from_center;

        self.dist_from_center += 1;
        self.mul *= 1;
        Some(self.list[idx as usize].clone())
    }
}

pub(crate) fn iter_by_importance(range_list: Vec<Range<u64>>) -> RangeListIterator {
    RangeListIterator {
        list: range_list,
        dist_from_center: 0,
        mul: 1,
    }
}

pub(crate) trait RangeLen {
    fn len(&self) -> u64;
}

impl RangeLen for Range<u64> {
    fn len(&self) -> u64 {
        self.end - self.start
    }
}

pub(crate) fn correct_for_capacity(
    needed_from_src: Vec<Range<u64>>,
    target: &mut Store,
) -> RangeSet<u64> {
    use crate::store::CapacityBounds;
    let CapacityBounds::Limited(capacity) = target.capacity().total() else {
        return RangeSet::from_iter(needed_from_src.into_iter());
    };

    let mut free_capacity = capacity.get();
    let mut res = RangeSet::new();

    for mut range in iter_by_importance(needed_from_src) {
        if range.len() <= free_capacity {
            res.insert(range.clone());
            free_capacity -= range.len();
        } else {
            range.start = range.end - free_capacity;
            res.insert(range.clone());
            break;
        }
    }
    res
}

/// Get up to the number of ranges supported by the target around the
/// currently being read range. Prioritizes the currently being read range
/// and the ranges after it.
#[instrument(level="trace", skip_all, ret)]
pub(super) fn ranges_we_can_take(src: &Store, target: &Store) -> Vec<Range<u64>> {
    let range_list: Vec<Range<u64>> = src.ranges().iter().cloned().collect();

    let res = range_list.binary_search_by_key(&src.last_read_pos(), |range| range.start);
    let range_currently_being_read = match res {
        Ok(pos_is_range_start) => pos_is_range_start,
        Err(pos_is_after_range_start) => pos_is_after_range_start,
    };

    let mut taking = Vec::with_capacity(range_list.len());

    let center = range_currently_being_read;
    let start = center;
    let end = range_list.len().min(center + target.n_supported_ranges());
    taking.extend_from_slice(&range_list[start..end]);

    let n_left = target.n_supported_ranges().saturating_sub(taking.len());
    let end = center;
    let start = center.saturating_sub(n_left);
    taking.extend_from_slice(&range_list[start..end]);

    taking
}
