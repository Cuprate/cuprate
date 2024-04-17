//! # Concurrent Map Builder
//!
//! This crate provides a [`TODO`](), that allows a single thread to partially initialise a Map with keys
//! needed and allows the retrieval of the corresponding values to be done my many threads.
//!
//! In the context of a database this means that a thread could create a [`TODO`](), for keys needed,
//! pass the [`ConcurrentBuilders`] to many DB workers who can concurrently work on getting the corresponding values.
//!
//! This allows us to do optimisations not possible for other concurrent maps as we know:
//! - The exact size of the Map
//! - Each worker will only add to a map
//! - The keys that will be inserted concurrently.
//!

use std::sync::Arc;

use indexmap::IndexSet;

mod builder;

use builder::ConcurrentMapBuilder;

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum ConcurrentMapBuilderError {
    #[error("A builder dropped [`MapBuilderWork`] before all values were inserted.")]
    WorkWasDroppedBeforeInsertingAllValues,
    #[error("A call to finish was made before all work was handed out.")]
    WorkWasNotFinishedBeforeInit,
}

#[derive(Debug)]
pub struct BuiltMap<K, V> {
    index_set: IndexSet<K>,
    values: Vec<V>,
}

impl<K, V> BuiltMap<K, V> {
    pub fn builder(keys_needed: IndexSet<K>) -> ConcurrentMapBuilder<K, V> {
        ConcurrentMapBuilder(Arc::new(builder::SharedConcurrentMapBuilder::new(
            keys_needed,
        )))
    }
}

#[test]
fn build() {
    use std::time::Duration;

    let mut keys = IndexSet::new();
    keys.extend(0..1000_u16);

    let map_builder = BuiltMap::<u16, u16>::builder(keys);

    let map_builder2 = map_builder.clone();

    let handle = std::thread::spawn(move || loop {
        let Some(mut work) = map_builder2.get_work(5).unwrap() else {
            return;
        };

        let keys_needed = work.keys_needed();

        for key in keys_needed {
            println!("Thread1: {}", key);
            work.insert_next_value(*key).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    let map_builder3 = map_builder.clone();

    let handle2 = std::thread::spawn(move || loop {
        let Some(mut work) = map_builder3.get_work(5).unwrap() else {
            return;
        };

        let keys_needed = work.keys_needed();

        for key in keys_needed {
            println!("Thread2: {}", key);
            work.insert_next_value(*key).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    let map_builder4 = map_builder.clone();

    let handle3 = std::thread::spawn(move || loop {
        let Some(mut work) = map_builder4.get_work(5).unwrap() else {
            return;
        };

        let keys_needed = work.keys_needed();

        for key in keys_needed {
            println!("Thread3: {}", key);
            work.insert_next_value(*key).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    handle.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    let map = map_builder.try_finish().unwrap().unwrap();

    println!("{:?}", map.values);
}
