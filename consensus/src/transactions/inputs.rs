use curve25519_dalek::EdwardsPoint;
use std::cmp::{max, min};
use std::collections::HashSet;

use monero_serai::transaction::Input;
use tower::{Service, ServiceExt};

use crate::{hardforks::HardFork, ConsensusError, Database, DatabaseRequest, DatabaseResponse};

/// A struct holding information about the inputs and their decoys.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html
pub struct DecoyInfo {
    /// The number of inputs that have enough outputs on the chain to mix with.
    mixable: usize,
    /// The number of inputs that don't have enough outputs on the chain to mix with.
    not_mixable: usize,
    /// The minimum amount of decoys used in the transaction.
    min_decoys: usize,
    /// The maximum amount of decoys used in the transaction.
    max_decoys: usize,
}

impl DecoyInfo {
    /// Creates a new [`DecoyInfo`] struct relating to the passed in inputs.
    ///
    /// Do not rely on this function to do consensus checks!
    ///
    pub async fn new<D: Database>(
        inputs: &[Input],
        hf: &HardFork,
        mut database: D,
    ) -> Result<DecoyInfo, ConsensusError> {
        let mut min_decoys = usize::MAX;
        let mut max_decoys = usize::MIN;
        let mut mixable = 0;
        let mut not_mixable = 0;

        let minimum_decoys = minimum_decoys(hf);

        for inp in inputs {
            match inp {
                Input::ToKey {
                    amount,
                    key_offsets,
                    ..
                } => {
                    let DatabaseResponse::NumberOutputsWithAmount(numb_of_outs) = database
                        .ready()
                        .await?
                        .call(DatabaseRequest::NumberOutputsWithAmount(
                            amount.unwrap_or(0),
                        ))
                        .await?
                    else {
                        panic!("Database sent incorrect response!");
                    };

                    // https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html#mixable-and-unmixable-inputs
                    if numb_of_outs <= minimum_decoys {
                        not_mixable += 1;
                    } else {
                        mixable += 1;
                    }

                    let numb_decoys = key_offsets
                        .len()
                        .checked_sub(1)
                        .ok_or(ConsensusError::TransactionHasInvalidRing("ring is empty"))?;
                    // https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html#minimum-and-maximum-decoys-used
                    min_decoys = min(min_decoys, numb_decoys);
                    max_decoys = max(max_decoys, numb_decoys);
                }
                _ => {
                    return Err(ConsensusError::TransactionHasInvalidInput(
                        "input not ToKey",
                    ))
                }
            }
        }

        Ok(DecoyInfo {
            mixable,
            not_mixable,
            min_decoys,
            max_decoys,
        })
    }

    /// Checks the decoys are allowed.
    ///
    /// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#minimum-decoys
    /// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#equal-number-of-decoys
    pub fn check_decoy_info(&self, hf: &HardFork) -> Result<(), ConsensusError> {
        if hf == &HardFork::V15 {
            // Hard-fork 15 allows both v14 and v16 rules
            return self
                .check_decoy_info(&HardFork::V14)
                .or_else(|_| self.check_decoy_info(&HardFork::V16));
        }

        let current_minimum_decoys = minimum_decoys(hf);

        if self.min_decoys < current_minimum_decoys {
            if self.not_mixable == 0 {
                return Err(ConsensusError::TransactionHasInvalidRing(
                    "input does not have enough decoys",
                ));
            }
            if self.mixable > 1 {
                return Err(ConsensusError::TransactionHasInvalidInput(
                    "more than one mixable input with unmixable inputs",
                ));
            }
        }

        if hf >= &HardFork::V8 && self.min_decoys != current_minimum_decoys {
            return Err(ConsensusError::TransactionHasInvalidRing(
                "one ring does not have the minimum number of decoys",
            ));
        }

        if hf >= &HardFork::V12 && self.min_decoys != self.max_decoys {
            return Err(ConsensusError::TransactionHasInvalidRing(
                "rings do not have the same number of members",
            ));
        }

        Ok(())
    }

    /// Checks the version is in the allowed range.
    ///
    /// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#version
    pub fn check_tx_version(&self, version: u64, hf: &HardFork) -> Result<(), ConsensusError> {
        if version == 0 {
            return Err(ConsensusError::TransactionVersionInvalid);
        }

        let max = max_tx_version(hf);
        if version > max {
            return Err(ConsensusError::TransactionVersionInvalid);
        }

        // TODO: Doc is wrong here
        let min = min_tx_version(hf);
        if version < min && self.not_mixable != 0 {
            return Err(ConsensusError::TransactionVersionInvalid);
        }

        Ok(())
    }
}

fn max_tx_version(hf: &HardFork) -> u64 {
    if hf <= &HardFork::V3 {
        1
    } else {
        2
    }
}

fn min_tx_version(hf: &HardFork) -> u64 {
    if hf >= &HardFork::V6 {
        2
    } else {
        1
    }
}

/// Returns the minimum amount of decoys for a hard-fork.
/// **There are exceptions to this always being the minimum decoys**
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html#minimum-amount-of-decoys
fn minimum_decoys(hf: &HardFork) -> usize {
    use HardFork::*;
    match hf {
        V1 => panic!("hard-fork 1 does not use these rules!"),
        V2 | V3 | V4 | V5 => 2,
        V6 => 4,
        V7 => 6,
        V8 | V9 | V10 | V11 | V12 | V13 | V14 => 10,
        _ => 15,
    }
}

/// Sums the inputs checking for overflow.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#inputs-and-outputs-must-not-overflow
pub(crate) fn sum_inputs_v1(inputs: &[Input]) -> Result<u64, ConsensusError> {
    let mut sum: u64 = 0;
    for inp in inputs {
        match inp {
            Input::ToKey { amount, .. } => {
                sum = sum
                    .checked_add(amount.unwrap_or(0))
                    .ok_or(ConsensusError::TransactionInputsOverflow)?;
            }
            _ => {
                return Err(ConsensusError::TransactionHasInvalidInput(
                    "input not ToKey",
                ))
            }
        }
    }

    Ok(sum)
}

/// Checks the inputs key images for torsion and for duplicates in the transaction.
///
/// The `spent_kis` parameter is not meant to be a complete list of key images, just a list of related transactions
/// key images, for example transactions in a block. The chain will be checked for duplicates later.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#unique-key-image
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#torsion-free-key-image
pub(crate) fn check_key_images(
    inputs: &[Input],
    spent_kis: &mut HashSet<[u8; 32]>,
) -> Result<(), ConsensusError> {
    for inp in inputs {
        match inp {
            Input::ToKey { key_image, .. } => {
                if !key_image.is_torsion_free() {
                    return Err(ConsensusError::TransactionHasInvalidInput(
                        "key image has torsion",
                    ));
                }
                if !spent_kis.insert(key_image.compress().to_bytes()) {
                    return Err(ConsensusError::TransactionHasInvalidInput(
                        "key image already spent",
                    ));
                }
            }
            _ => {
                return Err(ConsensusError::TransactionHasInvalidInput(
                    "Input not ToKey",
                ))
            }
        }
    }

    Ok(())
}
