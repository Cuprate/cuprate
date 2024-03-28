//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

//---------------------------------------------------------------------------------------------------- Config
/// TODO
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {}

impl Config {
    /// Create a new [`Config`] with sane default settings.
    pub fn new() -> Self {
        todo!()
    }
}

impl Default for Config {
    /// Same as `Self::new(None)`.
    ///
    /// ```rust
    /// # use cuprate_database_benchmark::config::*;
    /// assert_eq!(Config::default(), Config::new(None));
    /// ```
    fn default() -> Self {
        Self::new()
    }
}
