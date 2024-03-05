use std::{
    cell::UnsafeCell,
    cmp::min,
    hash::Hash,
    mem::{needs_drop, MaybeUninit},
    ops::Range,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, OnceLock,
    },
};

use indexmap::{set::Slice, Equivalent, IndexSet};

use crate::{BuiltMap, ConcurrentMapBuilderError};

/// The shared part of the ConcurrentMapBuilder, this holds data that is needed by each worker.
#[derive(Debug)]
pub(crate) struct SharedConcurrentMapBuilder<K, V> {
    /// The set of keys we are building for.
    index_set: Option<IndexSet<K>>,
    /// The index of the last value that has a builder.
    current_index: AtomicUsize,

    /// Values that we are initialising, will be the length of `index_set`.
    ///
    /// The index for a keys value is given by the keys index in `index_set`.
    values: Option<Vec<UnsafeCell<MaybeUninit<V>>>>,
    /// A marker for if a value in `values` is initialised.
    initialised_values: Vec<UnsafeCell<bool>>,

    /// An error slot that is shared between builders.
    error_slot: OnceLock<ConcurrentMapBuilderError>,
}

// We are only allowing one thread to mutate a value.
// TODO: I don't know if we need the sync bounds on K, V.
unsafe impl<K: Sync, V: Sync> Sync for SharedConcurrentMapBuilder<K, V> {}

impl<K, V> SharedConcurrentMapBuilder<K, V> {
    /// Returns a new [`SharedConcurrentMapBuilder`], with the keys needed in an [`IndexSet`].
    pub fn new(keys_needed: IndexSet<K>) -> SharedConcurrentMapBuilder<K, V> {
        let values = Some(
            (0..keys_needed.len())
                .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
                .collect(),
        );
        let initialised_values = (0..keys_needed.len())
            .map(|_| UnsafeCell::new(false))
            .collect();

        SharedConcurrentMapBuilder {
            index_set: Some(keys_needed),
            current_index: AtomicUsize::new(0),
            values,
            initialised_values,
            error_slot: OnceLock::new(),
        }
    }
}

impl<K, V> Drop for SharedConcurrentMapBuilder<K, V> {
    fn drop(&mut self) {
        // Values in a MaybeUninit will not be dropped so we need to drop them manually.

        // This will only be ran when all workers have dropped their handles.
        if needs_drop::<V>() {
            if let Some(values) = &self.values {
                for init_value in self
                    .initialised_values
                    .iter()
                    .zip(values.iter())
                    .filter(|(flag, _)| unsafe {
                        // SAFETY:
                        // We are running drop code - this is the only reference.
                        *flag.get()
                    })
                    .map(|(_, v)| v)
                {
                    // SAFETY:
                    // We are running drop code - this is the only reference.
                    let value = unsafe { &mut *init_value.get() };

                    // SAFETY:
                    // This value had the init flag set to initialised.
                    unsafe { value.assume_init_drop() }
                }
            }
        }
    }
}

/// A builder that can be cloned and handed out to multiple threads to construct a [`BuiltMap`].
#[derive(Debug, Clone)]
pub struct ConcurrentMapBuilder<K, V>(pub(crate) Arc<SharedConcurrentMapBuilder<K, V>>);

impl<K, V> ConcurrentMapBuilder<K, V> {
    /// Returns [`MapBuilderWork`] which allows adding some values for specific keys.
    ///
    /// The amount of keys which are asked for will be less than or equal to `amt`.
    ///
    /// Returns Ok(None) if there is no more work left.
    pub fn get_work(
        &self,
        amt: usize,
    ) -> Result<Option<MapBuilderWork<'_, K, V>>, ConcurrentMapBuilderError> {
        // This unwrap is safe as it will only be None when `try_finish` is called.
        let values = self.0.values.as_ref().unwrap();

        if let Some(err) = self.0.error_slot.get() {
            return Err(*err);
        }

        // TODO: can we use a weaker Ordering?
        let start = self.0.current_index.fetch_add(amt, Ordering::SeqCst);

        if start >= values.len() {
            // No work to do, all given out.
            return Ok(None);
        }

        let end = min(start + amt, values.len());

        Ok(Some(MapBuilderWork {
            index_set: self.0.index_set.as_ref().unwrap(),
            work_range: start..end,
            current_local_index: 0,
            values: &values[start..end],
            initialised_values: &self.0.initialised_values[start..end],
            error_slot: &self.0.error_slot,
        }))
    }

    pub fn try_finish(self) -> Result<Option<BuiltMap<K, V>>, ConcurrentMapBuilderError> {
        // Check if we are the only one holding the Arc.
        let Some(mut inner) = Arc::into_inner(self.0) else {
            // Another thread will finish.
            return Ok(None);
        };

        if let Some(err) = inner.error_slot.get() {
            return Err(*err);
        }

        let values = inner.values.take().unwrap();

        if inner.current_index.load(Ordering::Relaxed) < values.len() {
            return Err(ConcurrentMapBuilderError::WorkWasNotFinishedBeforeInit);
        }

        // SAFETY:
        // - UnsafeCell<MaybeUninit<T>> has the same bit pattern as T.
        // - If any value is unitised that means work wasn't handed out which we just
        //   checked for, or work handed out was not completed which is checked for in
        //   the Drop impl of MapBuilderWork.
        let values: Vec<V> = unsafe { std::mem::transmute(values) };

        Ok(Some(BuiltMap {
            index_set: inner.index_set.take().unwrap(),
            values,
        }))
    }
}

#[derive(Debug)]
pub struct MapBuilderWork<'a, K, V> {
    /// The set of keys we are building for.
    index_set: &'a IndexSet<K>,
    /// The range of values we are currently building.
    work_range: Range<usize>,

    /// The local index of the next value to build in `values`.
    current_local_index: usize,
    /// The values in the range we are initialising.
    values: &'a [UnsafeCell<MaybeUninit<V>>],
    initialised_values: &'a [UnsafeCell<bool>],
    /// An error slot that is shared between builders.
    error_slot: &'a OnceLock<ConcurrentMapBuilderError>,
}

impl<'a, K, V> Drop for MapBuilderWork<'a, K, V> {
    fn drop(&mut self) {
        if self.current_local_index != self.work_range.end - self.work_range.start {
            let _ = self
                .error_slot
                .set(ConcurrentMapBuilderError::WorkWasDroppedBeforeInsertingAllValues);
        }
    }
}

impl<'a, K, V> MapBuilderWork<'a, K, V>
where
    K: Hash + Equivalent<K>,
{
    /// This function returns all the keys that need to be got by this worker.
    ///
    /// If the worker fails to get all the keys then, the whole build fails.
    #[inline]
    pub fn keys_needed(&self) -> &'a Slice<K> {
        // TODO: remove clones for work_range
        self.index_set.get_range(self.work_range.clone()).unwrap()
    }

    /// Inserts the next value into the Map.
    ///
    /// Values must be inserted in the same order their keys are returned in [`MapBuilderWork::keys_needed`].#
    ///
    /// An error is returned if another worker failed to insert all of their values.
    #[inline]
    pub fn insert_next_value(&mut self, value: V) -> Result<(), ConcurrentMapBuilderError> {
        assert!(self.current_local_index < self.work_range.end);

        if let Some(err) = self.error_slot.get() {
            return Err(*err);
        }

        let index = self.current_local_index;
        // SAFETY:
        // When we got keys from the [`ConcurrentMapBuilder`] we used an atomic operation
        // to make sure our range of values we are building are unique.
        let value_slot = unsafe { &mut *self.values[index].get() };
        let init_flag_slot = unsafe { &mut *self.initialised_values[index].get() };

        value_slot.write(value);
        *init_flag_slot = true;

        self.current_local_index += 1;

        Ok(())
    }
}
