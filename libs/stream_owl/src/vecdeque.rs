use std::collections::VecDeque;

pub trait VecDequeExt {
    fn copy_starting_at(&self, start: usize, target: &mut [u8]) -> usize;
}

impl VecDequeExt for VecDeque<u8> {
    /// Copies from this vecdeque into a provided slice in at most two memcopys.
    /// You can provide an optional start to skip the first n_items in the vecdeque.
    ///
    /// # Panic:
    fn copy_starting_at(&self, start: usize, target: &mut [u8]) -> usize {
        let (front, back) = self.as_slices();
        if front.len() >= start {
            let n_to_copy = front.len() - start;
            let n_to_copy = n_to_copy.min(target.len());
            target[..n_to_copy].copy_from_slice(&front[start..start + n_to_copy]);
            let n_copied = n_to_copy;

            // copy remaining needed bytes from back
            let n_to_copy = target.len().saturating_sub(n_copied);
            let n_to_copy = n_to_copy.min(back.len());
            target[n_copied..n_copied + n_to_copy].copy_from_slice(&back[..n_to_copy]);

            n_copied + n_to_copy
        } else {
            let n_to_copy = back.len() - start;
            let n_to_copy = n_to_copy.min(target.len());
            target[..n_to_copy].copy_from_slice(&back[start..start + n_to_copy]);
            let n_copied = n_to_copy;

            n_copied
        }
    }
}

#[macro_export]
macro_rules! vecd {
    [$elem:expr; $n:expr] => {
        {
            let mut dvec = VecDeque::with_capacity($n);
            for _ in 0..$n {
                dvec.push_back($elem);
            }
            dvec
        }
    };
    [$($x:expr),+$(; $($y:expr),+)?] => {
        {
            let mut dvec = VecDeque::new();
            $(
            dvec.push_back($x);
            )+

            $(
            $(
            dvec.push_front($y);
            )+
            )?
            dvec
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod contiguous {
        use super::*;

        #[test]
        fn starting_at_zero() {
            let mut target = [0u8; 4];
            let buffer = vecd![1, 2, 3, 4];
            buffer.copy_starting_at(0, &mut target);
            assert_eq!(target, [1, 2, 3, 4]);
        }

        #[test]
        fn skip_half() {
            let mut target = [0u8; 2];
            let buffer = vecd![1, 2, 3, 4];
            buffer.copy_starting_at(2, &mut target);
            assert_eq!(target, [3, 4]);
        }

        #[test]
        fn target_bigger() {
            let mut target = [0u8; 6];
            let buffer = vecd![1, 2, 3, 4];
            buffer.copy_starting_at(0, &mut target);
            assert_eq!(target, [1, 2, 3, 4, 0, 0]);
        }

        #[test]
        fn target_smaller() {
            let mut target = [0u8; 2];
            let buffer = vecd![1, 2, 3, 4];
            buffer.copy_starting_at(0, &mut target);
            assert_eq!(target, [1, 2]);
        }
    }

    #[cfg(test)]
    mod discontiguous {
        use super::*;

        #[test]
        fn starting_at_zero() {
            let mut target = [0u8; 8];
            let buffer = vecd![5,6,7,8; 4, 3, 2, 1];
            buffer.copy_starting_at(0, &mut target);
            assert_eq!(target, [1, 2, 3, 4, 5, 6, 7, 8]);
        }

        #[test]
        fn skip_half() {
            let mut target = [0u8; 4];
            let buffer = vecd![5,6,7,8; 4, 3, 2, 1];
            buffer.copy_starting_at(4, &mut target);
            assert_eq!(target, [5, 6, 7, 8]);
        }

        #[test]
        fn target_bigger() {
            let mut target = [0u8; 6];
            let buffer = vecd![5,6,7,8; 4, 3, 2, 1];
            buffer.copy_starting_at(4, &mut target);
            assert_eq!(target, [5, 6, 7, 8, 0, 0]);
        }

        #[test]
        fn target_smaller() {
            let mut target = [0u8; 2];
            let buffer = vecd![5,6,7,8; 4, 3, 2, 1];
            buffer.copy_starting_at(0, &mut target);
            assert_eq!(target, [1, 2]);
        }
    }
}
