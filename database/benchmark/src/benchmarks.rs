//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    collections::BTreeSet,
    path::{Path, PathBuf},
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
pub(crate) enum Benchmarks {
    /// TODO
    EnvOpen,
}

//---------------------------------------------------------------------------------------------------- Stats
/// TODO
#[derive(Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, Debug)]
struct Stats {
    /// TODO
    env_open: Option<f32>,
}

impl Stats {
    /// TODO
    const fn new() -> Self {
        Self { env_open: None }
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
    pub(crate) const fn new(env: ConcreteEnv, config: Config) -> Self {
        Self {
            env,
            config,
            stats: Stats::new(),
        }
    }

    /// TODO
    pub(crate) fn bench(self) {
        todo!()
    }

    /// TODO
    pub(crate) fn todo(&mut self) {
        println!("TODO");
    }
}
