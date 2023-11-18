use rangemap::set::RangeSet;

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

pub(crate) fn correct_for_capacity(needed_from_src: Vec<Range<u64>>, target: &mut Store) -> RangeSet<u64> {
    let Some(capacity) = target.capacity().total() else {
        return RangeSet::from_iter(needed_from_src.into_iter());
    };

    let mut free_capacity = capacity.get();
    let mut res = RangeSet::new();

    for mut range in iter_by_importance(needed_from_src) {
        if range.len() >= free_capacity {
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

/// List is orderd by range start. The center element, calculated using
/// normal (flooring) devision by 2 is the current read pos.
pub(super) fn needed_ranges(src: &Store, target: &Store) -> Vec<Range<u64>> {
    let range_list: Vec<Range<u64>> = src.ranges().iter().cloned().collect();

    let search_res = range_list.binary_search_by_key(&src.last_read_pos(), |range| range.start);
    let idx = match search_res {
        Ok(pos_is_range_start) => pos_is_range_start,
        Err(pos_is_after_range_start) => pos_is_after_range_start,
    };

    let needed_ranges = fixed_window(&range_list, target.n_supported_ranges(), idx);
    needed_ranges.to_vec()
}

fn fixed_window<T>(slice: &[T], center: usize, window: usize) -> &[T] {
    let space_before = center;
    let space_ahead = slice.len() - center - 1;

    let n_before;
    let n_after;

    let half_window = window.div_ceil(2);
    if center < slice.len() / 2 {
        // bigger chance that there is space ahead of the center then before
        n_before = usize::min(space_before, half_window - 1);
        n_after = usize::min(space_ahead, window - n_before - 1);
    } else {
        n_after = usize::min(space_ahead, half_window - 1);
        n_before = usize::min(space_before, window - n_after - 1);
    }

    &slice[center - n_before..=center + n_after]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_squishy_window() {
        let big = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let res = fixed_window(&big, 5, 5);
        assert_eq!(res, &[3, 4, 5, 6, 7]);
        let res = fixed_window(&big, 0, 5);
        assert_eq!(res, &[0, 1, 2, 3, 4]);
        let res = fixed_window(&big, 10, 5);
        assert_eq!(res, &[6, 7, 8, 9, 10]);

        let res = fixed_window(&big, 10, 1);
        assert_eq!(res, &[10]);
        let res = fixed_window(&big, 0, 1);
        assert_eq!(res, &[0]);
        let res = fixed_window(&big, 5, 1);
        assert_eq!(res, &[5]);

        let tiny = [0, 1, 2];
        let res = fixed_window(&tiny, 1, 1);
        assert_eq!(res, &[1]);
        let res = fixed_window(&tiny, 0, 1);
        assert_eq!(res, &[0]);
        let res = fixed_window(&tiny, 2, 1);
        assert_eq!(res, &[2]);

        let res = fixed_window(&tiny, 1, 10);
        assert_eq!(res, &[0, 1, 2]);
    }
}
