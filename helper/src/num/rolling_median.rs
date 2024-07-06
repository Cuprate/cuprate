use std::{
    collections::VecDeque,
    ops::{Add, Div, Mul, Sub},
};

use crate::num::median;

/// A rolling median type.
///
/// The `RollingMedian` keeps track of window of items and allows calculating the [RollingMedian::median] of them.
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
    /// The target window is the maximum amount of items to keep in the rolling window.
    pub fn new(target_window: usize) -> RollingMedian<T> {
        RollingMedian {
            window: VecDeque::with_capacity(target_window),
            sorted_window: Vec::with_capacity(target_window),
            target_window,
        }
    }

    /// Creates a new [`RollingMedian`] from a [`Vec`] with a certain target window length.
    ///
    /// The target window is the maximum amount of items to keep in the rolling window.
    ///
    /// # Panics
    /// This function panics if the vec is larger than the target window length.
    pub fn from_vec(value: Vec<T>, target_window: usize) -> RollingMedian<T> {
        assert!(value.len() <= target_window);

        let mut sorted_window = value.clone();
        sorted_window.sort_unstable();

        RollingMedian {
            window: value.into(),
            sorted_window,
            target_window,
        }
    }

    /// Pops the front of the window, i.e. the oldest item.
    ///
    /// This is often not needed [`RollingMedian::push`] will handle popping old values when they fall
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

    /// Calculated the median of the values currently in the [`RollingMedian`].
    pub fn median(&self) -> T {
        median(&self.sorted_window)
    }
}
