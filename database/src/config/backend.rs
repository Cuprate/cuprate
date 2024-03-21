//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_helper::fs::cuprate_database_dir;

use crate::{
    config::{ReaderThreads, SyncMode},
    constants::DATABASE_DATA_FILENAME,
    resize::ResizeAlgorithm,
};

//---------------------------------------------------------------------------------------------------- Backend
/// TODO
#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Backend {
    #[default]
    /// TODO
    Heed,
    /// TODO
    Redb,
}
