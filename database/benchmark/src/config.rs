//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::{bench::Benchmark, cli::Cli};

//---------------------------------------------------------------------------------------------------- Config
/// TODO
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
pub(crate) struct Config {
    /// TODO
    pub(crate) iterations: usize,
    /// TODO
    pub(crate) benchmark_set: BTreeSet<Benchmark>,
}

impl Config {
    /// Create a new [`Config`] with sane default settings.
    pub(crate) fn new() -> Self {
        Self {
            iterations: 100_000,
            benchmark_set: Benchmark::iter().collect(),
        }
    }

    /// TODO
    pub(crate) fn merge(&mut self, cli: &Cli) {
        todo!()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
