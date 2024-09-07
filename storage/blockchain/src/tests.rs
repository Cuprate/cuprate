//! Utilities for `cuprate_blockchain` testing.
//!
//! These types/fn's are only:
//! - enabled on #[cfg(test)]
//! - only used internally

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, fmt::Debug};

use pretty_assertions::assert_eq;

use cuprate_database::{ConcreteEnv, DatabaseRo, Env, EnvInner};
use cuprate_types::{AltBlockInformation, ChainId, VerifiedBlockInformation};

use crate::{
    config::ConfigBuilder,
    tables::{OpenTables, Tables},
};

//---------------------------------------------------------------------------------------------------- Struct
/// Named struct to assert the length of all tables.
///
/// This is a struct with fields instead of a function
/// so that callers can name arguments, otherwise the call-site
/// is a little confusing, i.e. `assert_table_len(0, 25, 1, 123)`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AssertTableLen {
    pub(crate) block_infos: u64,
    pub(crate) block_blobs: u64,
    pub(crate) block_heights: u64,
    pub(crate) key_images: u64,
    pub(crate) num_outputs: u64,
    pub(crate) pruned_tx_blobs: u64,
    pub(crate) prunable_hashes: u64,
    pub(crate) outputs: u64,
    pub(crate) prunable_tx_blobs: u64,
    pub(crate) rct_outputs: u64,
    pub(crate) tx_blobs: u64,
    pub(crate) tx_ids: u64,
    pub(crate) tx_heights: u64,
    pub(crate) tx_unlock_time: u64,
}

impl AssertTableLen {
    /// Assert the length of all tables.
    pub(crate) fn assert(self, tables: &impl Tables) {
        let other = Self {
            block_infos: tables.block_infos().len().unwrap(),
            block_blobs: tables.block_blobs().len().unwrap(),
            block_heights: tables.block_heights().len().unwrap(),
            key_images: tables.key_images().len().unwrap(),
            num_outputs: tables.num_outputs().len().unwrap(),
            pruned_tx_blobs: tables.pruned_tx_blobs().len().unwrap(),
            prunable_hashes: tables.prunable_hashes().len().unwrap(),
            outputs: tables.outputs().len().unwrap(),
            prunable_tx_blobs: tables.prunable_tx_blobs().len().unwrap(),
            rct_outputs: tables.rct_outputs().len().unwrap(),
            tx_blobs: tables.tx_blobs().len().unwrap(),
            tx_ids: tables.tx_ids().len().unwrap(),
            tx_heights: tables.tx_heights().len().unwrap(),
            tx_unlock_time: tables.tx_unlock_time().len().unwrap(),
        };

        assert_eq!(self, other);
    }
}

//---------------------------------------------------------------------------------------------------- fn
/// Create an `Env` in a temporarily directory.
/// The directory is automatically removed after the `TempDir` is dropped.
///
/// FIXME: changing this to `-> impl Env` causes lifetime errors...
pub(crate) fn tmp_concrete_env() -> (ConcreteEnv, tempfile::TempDir) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = ConfigBuilder::new()
        .db_directory(Cow::Owned(tempdir.path().into()))
        .low_power()
        .build();
    let env = crate::open(config).unwrap();

    (env, tempdir)
}

/// Assert all the tables in the environment are empty.
pub(crate) fn assert_all_tables_are_empty(env: &ConcreteEnv) {
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let tables = env_inner.open_tables(&tx_ro).unwrap();
    assert!(tables.all_tables_empty().unwrap());
    assert_eq!(crate::ops::tx::get_num_tx(tables.tx_ids()).unwrap(), 0);
}

pub(crate) fn map_verified_block_to_alt(
    verified_block: VerifiedBlockInformation,
    chain_id: ChainId,
) -> AltBlockInformation {
    AltBlockInformation {
        block: verified_block.block,
        block_blob: verified_block.block_blob,
        txs: verified_block.txs,
        block_hash: verified_block.block_hash,
        pow_hash: verified_block.pow_hash,
        height: verified_block.height,
        weight: verified_block.weight,
        long_term_weight: verified_block.long_term_weight,
        cumulative_difficulty: verified_block.cumulative_difficulty,
        chain_id,
    }
}
