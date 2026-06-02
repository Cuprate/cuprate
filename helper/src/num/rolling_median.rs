use std::{
    cmp::min,
    collections::VecDeque,
    ops::{Add, Div, Mul, Sub},
};

use crate::num::{get_mid, median};

/// A rolling median type.
///
/// This keeps track of a window of items and allows calculating the [`RollingMedian::median`] of them.
///
/// Example:
/// ```rust
/// # use cuprate_helper::num::RollingMedian;
/// let mut rolling_median = RollingMedian::new(2);
///
/// rolling_median.push(1);
/// assert_eq!(rolling_median.median(), 1);
/// assert_eq!(rolling_median.window_len(), 1);
///
/// rolling_median.push(3);
/// assert_eq!(rolling_median.median(), 2);
/// assert_eq!(rolling_median.window_len(), 2);
///
/// rolling_median.push(5);
/// assert_eq!(rolling_median.median(), 4);
/// assert_eq!(rolling_median.window_len(), 2);
/// ```
///
// TODO: a more efficient structure is probably possible.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
pub struct RollingMedian<T> {
    /// The window of items, in order of insertion.
    window: VecDeque<T>,
    /// The window of items, sorted.
    sorted_window: Vec<T>,

    /// The target window length.
    target_window: usize,
}

impl<T> RollingMedian<T>
where
    T: Ord
        + PartialOrd
        + Add<Output = T>
        + Sub<Output = T>
        + Div<Output = T>
        + Mul<Output = T>
        + Copy
        + From<u8>,
{
    /// Creates a new [`RollingMedian`] with a certain target window length.
    ///
    /// `target_window` is the maximum amount of items to keep in the rolling window.
    pub fn new(target_window: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(target_window),
            sorted_window: Vec::with_capacity(target_window),
            target_window,
        }
    }

    /// Creates a new [`RollingMedian`] from a [`Vec`] with a certain target window length.
    ///
    /// `target_window` is the maximum amount of items to keep in the rolling window.
    ///
    /// # Panics
    /// This function panics if `vec.len() > target_window`.
    pub fn from_vec(vec: Vec<T>, target_window: usize) -> Self {
        assert!(vec.len() <= target_window);

        let mut sorted_window = vec.clone();
        sorted_window.sort_unstable();

        Self {
            window: vec.into(),
            sorted_window,
            target_window,
        }
    }

    /// Pops the front of the window, i.e. the oldest item.
    ///
    /// This is often not needed as [`RollingMedian::push`] will handle popping old values when they fall
    /// out of the window.
    pub fn pop_front(&mut self) {
        if let Some(item) = self.window.pop_front() {
            match self.sorted_window.binary_search(&item) {
                Ok(idx) => {
                    self.sorted_window.remove(idx);
                }
                Err(_) => panic!("Value expected to be in sorted_window was not there"),
            }
        }
    }

    /// Pops the back of the window, i.e. the youngest item.
    pub fn pop_back(&mut self) {
        if let Some(item) = self.window.pop_back() {
            match self.sorted_window.binary_search(&item) {
                Ok(idx) => {
                    self.sorted_window.remove(idx);
                }
                Err(_) => panic!("Value expected to be in sorted_window was not there"),
            }
        }
    }

    /// Push an item to the _back_ of the window.
    ///
    /// This will pop the oldest item in the window if the target length has been exceeded.
    pub fn push(&mut self, item: T) {
        if self.window.len() >= self.target_window {
            self.pop_front();
        }

        self.window.push_back(item);
        match self.sorted_window.binary_search(&item) {
            Ok(idx) | Err(idx) => self.sorted_window.insert(idx, item),
        }
    }

    /// Append some values to the _front_ of the window.
    ///
    /// These new values will be the oldest items in the window. The order of the inputted items will be
    /// kept, i.e. the first item in the [`Vec`] will be the oldest item in the queue.
    pub fn append_front(&mut self, items: Vec<T>) {
        for item in items.into_iter().rev() {
            self.window.push_front(item);
            match self.sorted_window.binary_search(&item) {
                Ok(idx) | Err(idx) => self.sorted_window.insert(idx, item),
            }

            if self.window.len() > self.target_window {
                self.pop_back();
            }
        }
    }

    /// Returns the number of items currently in the [`RollingMedian`].
    pub fn window_len(&self) -> usize {
        self.window.len()
    }

    /// Calculates the median of the values currently in the [`RollingMedian`].
    pub fn median(&self) -> T {
        median(&self.sorted_window)
    }

    /// Calculates a median value with a set amount of `grace` values.
    ///
    /// `grace` values are minimum values added to the back of the [`RollingMedian`]. The median is then
    /// got as if these values had been added and replaced any values at the front, if the capacity is
    /// reached.
    pub fn median_with_grace(&self, grace: usize) -> T {
        let zero = T::from(0);
        let current_len = self.sorted_window.len();
        let cap = self.target_window;

        // The amount of values that would be dropped if this many grace values were added.
        let drop = (current_len + grace).saturating_sub(cap);
        // The new length of the window with the grace values.
        let new_len = min(current_len + grace, cap);

        if new_len == 0 || new_len / 2 < grace {
            return zero;
        }

        // The entries that would be removed if the grace values were added.
        let mut removed = self.window.iter().take(drop).copied().collect::<Vec<_>>();
        removed.sort_unstable();
        // An index into the sorted `removed` list.
        let mut rem_idx = 0;

        // Conceptual median index for the new window.
        let conceptual_idx = if new_len.is_multiple_of(2) {
            new_len / 2 - 1
        } else {
            new_len / 2
        };

        // The index into the real `sorted_window`.
        // Because the grace entries are not really in the window, we simulate them by shifting the median
        // search by grace entries. As the grace values are always `0`, it will shift the median down.
        let mut idx = conceptual_idx.saturating_sub(grace);

        // A closure to get the next live value, starting the search at the given index.
        // When we add grace values, some values may be removed from the list so we need to make sure
        // we are not using dead values to calculate the median.
        let next_live = |idx: &mut usize, rem_idx: &mut usize| -> T {
            loop {
                let v = self.sorted_window[*idx];
                // If the value we are currently looking at has a value more than or equal to a removed value we need to increase the median index.
                if removed.get(*rem_idx).is_some_and(|r| *r <= v) {
                    // Consume the removed value, we have now adjusted.
                    *rem_idx += 1;
                    // Increase the median index.
                    *idx += 1;
                    // Try the next value.
                    continue;
                }
                // We have found a value, increase the index for the next potential search.
                *idx += 1;
                return v;
            }
        };

        if new_len.is_multiple_of(2) {
            // This handles an edge case where the grace takes one of our median values as 0 but leaves
            // the other.
            let left = if conceptual_idx < grace {
                zero
            } else {
                next_live(&mut idx, &mut rem_idx)
            };

            get_mid(left, next_live(&mut idx, &mut rem_idx))
        } else {
            next_live(&mut idx, &mut rem_idx)
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::{collection::vec, prelude::*};

    use crate::num::RollingMedian;

    fn assert_median_with_grace(window: Vec<u64>, grace: usize, target_window: usize) {
        let mut median = RollingMedian::new(target_window);

        for i in window {
            median.push(i);
        }

        let median1 = median.median_with_grace(grace);

        for _ in 0..grace {
            median.push(0);
        }

        assert_eq!(median1, median.median());
    }

    proptest! {
        #[test]
        fn median_with_grace(window in vec(any::<u64>(), 1..10_000_usize), grace in 0..10_000_usize, target_window in 1..10_000_usize) {
            assert_median_with_grace(window, grace, target_window);
        }

        #[test]
        fn median_with_grace_tight(window in vec(0..50_u64, 1..10_000_usize), grace in 0..10_000_usize, target_window in 1..10_000_usize) {
            assert_median_with_grace(window, grace, target_window);
        }
    }
}
