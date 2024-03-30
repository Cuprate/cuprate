//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::Instant,
};

use serde::{Deserialize, Serialize};
use strum::{
    Display, EnumCount, EnumIs, EnumIter, EnumString, IntoStaticStr, VariantArray, VariantIterator,
};

use cuprate_database::ConcreteEnv;

use crate::config::Config;

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
#[derive(
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Serialize,
    Deserialize,
    Debug,
    Hash,
    Display,
    EnumCount,
    EnumIs,
    EnumIter,
    EnumString,
    IntoStaticStr,
    VariantArray,
)]
pub(crate) enum Benchmark {
    /// Maps to [`env_open`].
    EnvOpen,
}

impl Benchmark {
    /// Map [`Benchmark`] to the proper benchmark function.
    #[inline(always)]
    fn benchmark_fn(self) -> fn(&ConcreteEnv) {
        match self {
            Self::EnvOpen => env_open,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Stats
/// TODO
#[derive(Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, Debug)]
struct Stats {
    /// Timings for [`env_open`].
    env_open: Option<f32>,
}

impl Stats {
    /// Create a new [`Stats`] with no benchmark timings.
    const fn new() -> Self {
        Self { env_open: None }
    }

    /// This maps [`Benchmark`]s to a specific field in [`Stats`]
    /// which then gets updated to the passed `time`.
    #[inline(always)]
    fn update_benchmark_time(&mut self, benchmark: Benchmark, time: f32) {
        *match benchmark {
            Benchmark::EnvOpen => &mut self.env_open,
        } = Some(time);
    }
}

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
pub(crate) struct Benchmarker {
    /// TODO
    env: ConcreteEnv,
    /// TODO
    config: Config,
    /// TODO
    stats: Stats,
}

//---------------------------------------------------------------------------------------------------- Drop
impl Drop for Benchmarker {
    fn drop(&mut self) {
        let stats_json = serde_json::to_string_pretty(&self.stats).unwrap();
        println!("{stats_json}");
    }
}

//---------------------------------------------------------------------------------------------------- Impl
impl Benchmarker {
    /// TODO
    #[cold]
    #[inline(never)]
    pub(crate) const fn new(env: ConcreteEnv, config: Config) -> Self {
        Self {
            env,
            config,
            stats: Stats::new(),
        }
    }

    /// Start all benchmark that are selected by the user.
    /// `main()` calls this once.
    #[cold]
    #[inline(never)]
    pub(crate) fn bench_all(mut self) {
        for benchmark in &self.config.benchmark_set {
            bench(*benchmark, &self.env, &mut self.stats);
        }
    }
}

//---------------------------------------------------------------------------------------------------- Benchmark functions
/// TODO
#[inline(always)]
fn bench(benchmark: Benchmark, env: &ConcreteEnv, stats: &mut Stats) {
    // Start the benchmark timer.
    let instant = Instant::now();

    // Benchmark.
    benchmark.benchmark_fn()(env);

    // Update the time.
    stats.update_benchmark_time(benchmark, instant.elapsed().as_secs_f32());
}

/// TODO
#[inline(always)]
fn env_open(env: &ConcreteEnv) {
    println!("TODO");
}
