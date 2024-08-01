//! SOMEDAY

//---------------------------------------------------------------------------------------------------- Import
//use std::{
//    borrow::Cow,
//    num::NonZeroUsize,
//    path::{Path, PathBuf},
//};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

//use cuprate_helper::fs::cuprate_blockchain_dir;

//use crate::{
//    config::{ReaderThreads, SyncMode},
//    constants::DATABASE_DATA_FILENAME,
//    resize::ResizeAlgorithm,
//};

//---------------------------------------------------------------------------------------------------- Backend
/// SOMEDAY: allow runtime hot-swappable backends.
#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Backend {
    #[default]
    /// SOMEDAY
    Heed,
    /// SOMEDAY
    Redb,
}
