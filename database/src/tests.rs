//! Utilities for `cuprate_database` testing.
//!
//! These types/fn's are only:
//! - enabled on #[cfg(test)]
//! - only used internally

//---------------------------------------------------------------------------------------------------- Import
use std::{fmt::Debug, sync::OnceLock};

use monero_serai::{
    ringct::{RctPrunable, RctSignatures},
    transaction::{Timelock, Transaction, TransactionPrefix},
};

use crate::{config::Config, key::Key, storable::Storable, ConcreteEnv, Env};

//---------------------------------------------------------------------------------------------------- Constants
/// TODO: This doesn't work due to (de)serialization assertions.
/// Figure out how to a real TX into `Transaction` type form.
///
/// Return a dummy `Transaction` struct.
///
/// - TX version is 2
/// - Most values are default, null, etc
/// - There is a timelock of 5
static DUMMY_TX: OnceLock<Transaction> = OnceLock::new();
pub(crate) fn dummy_tx() -> Transaction {
    DUMMY_TX
        .get_or_init(|| Transaction {
            prefix: TransactionPrefix {
                version: 2,
                timelock: Timelock::Time(5),
                inputs: Vec::default(),
                outputs: Vec::default(),
                extra: Vec::default(),
            },
            signatures: Vec::default(),
            rct_signatures: RctSignatures {
                base: monero_serai::ringct::RctBase {
                    fee: Default::default(),
                    pseudo_outs: Vec::default(),
                    encrypted_amounts: Vec::default(),
                    commitments: Vec::default(),
                },
                prunable: RctPrunable::Null,
            },
        })
        .clone()
}

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
