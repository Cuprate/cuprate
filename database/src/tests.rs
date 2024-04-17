//! Utilities for `cuprate_database` testing.
//!
//! These types/fn's are only:
//! - enabled on #[cfg(test)]
//! - only used internally

#![allow(clippy::significant_drop_tightening)]

//---------------------------------------------------------------------------------------------------- Import
use std::{
    fmt::Debug,
    sync::{Arc, OnceLock},
};

use monero_serai::{
    ringct::{RctPrunable, RctSignatures},
    transaction::{Timelock, Transaction, TransactionPrefix},
};

use cuprate_test_utils::data::{block_v16_tx0, block_v1_tx513, block_v9_tx3, tx_v2_rct3};
use cuprate_types::{TransactionVerificationData, VerifiedBlockInformation};

use crate::{
    config::Config, key::Key, storable::Storable, tables::Tables, transaction::TxRo, ConcreteEnv,
    Env, EnvInner,
};

//---------------------------------------------------------------------------------------------------- fn
/// Create an `Env` in a temporarily directory.
/// The directory is automatically removed after the `TempDir` is dropped.
///
/// FIXME: changing this to `-> impl Env` causes lifetime errors...
pub(crate) fn tmp_concrete_env() -> (ConcreteEnv, tempfile::TempDir) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = Config::low_power(Some(tempdir.path().into()));
    let env = ConcreteEnv::open(config).unwrap();

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

/// TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO
/// TODO: `cuprate_test_utils::data` should return `VerifiedBlockInformation`.
/// The tests below should be testing _real_ blocks that has _real_ transactions/outputs/hashes/etc.
///
/// As `VerifiedBlockInformation` contains some fields that
/// we cannot actually produce in `cuprate_database`, the testing
/// below will use not real but "close-enough" values.
///
/// For example, a real `pow_hash` is not computable here without
/// importing `PoW` code, so instead we fill it with dummy values.
pub(super) fn dummy_verified_block_information() -> VerifiedBlockInformation {
    let block = block_v9_tx3();

    // `pop_block()` finds and removes a block's transactions by its `block.txs` field
    // so we need to provide transactions that have the same hashes as `block_v9_tx3()`'s block.
    // The other contents are not real (fee, weight, etc).
    let tx = tx_v2_rct3();
    let mut txs = vec![];
    for tx_hash in [
        hex_literal::hex!("e2d39395dd1625b2d707b98af789e7eab9d24c2bd2978ec38ef910961a8cdcee"),
        hex_literal::hex!("e57440ec66d2f3b2a5fa2081af40128868973e7c021bb3877290db3066317474"),
        hex_literal::hex!("b6b4394d4ec5f08ad63267c07962550064caa8d225dd9ad6d739ebf60291c169"),
    ] {
        txs.push(Arc::new(TransactionVerificationData {
            tx_hash,
            tx: tx.clone(),
            tx_blob: tx.serialize(),
            tx_weight: tx.weight(),
            fee: 1_401_270_000,
        }));
    }

    VerifiedBlockInformation {
        block_hash: block.hash(),
        block_blob: block.serialize(),
        block,
        txs,                      // dummy
        pow_hash: [3; 32],        // dummy
        height: 0,                // dummy
        generated_coins: 3,       // dummy
        weight: 3,                // dummy
        long_term_weight: 3,      // dummy
        cumulative_difficulty: 3, // dummy
    }
}
