//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::{benchmarks::Benchmarks, cli::Cli};

//---------------------------------------------------------------------------------------------------- Config
/// TODO
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
pub struct Config {
    /// TODO
    iterations: usize,
    /// TODO
    benchmark_set: BTreeSet<Benchmarks>,
}

impl Config {
    /// Create a new [`Config`] with sane default settings.
    pub(crate) fn new() -> Self {
        Self {
            iterations: 100_000,
            benchmark_set: Benchmarks::iter().collect(),
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
