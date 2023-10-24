//! # Inputs
//!
//! This module contains all consensus rules for non-miner transaction inputs, excluding time locks.
//!

use std::{cmp::Ordering, collections::HashSet, sync::Arc};

use monero_serai::transaction::Input;

use crate::{
    transactions::{
        ring::{minimum_decoys, DecoyInfo, TxRingMembersInfo},
        TxVersion,
    },
    ConsensusError, HardFork,
};

/// Checks the decoys are allowed.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#minimum-decoys
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#equal-number-of-decoys
fn check_decoy_info(decoy_info: &DecoyInfo, hf: &HardFork) -> Result<(), ConsensusError> {
    if hf == &HardFork::V15 {
        // Hard-fork 15 allows both v14 and v16 rules
        return check_decoy_info(decoy_info, &HardFork::V14)
            .or_else(|_| check_decoy_info(decoy_info, &HardFork::V16));
    }

    let current_minimum_decoys = minimum_decoys(hf);

    if decoy_info.min_decoys < current_minimum_decoys {
        if decoy_info.not_mixable == 0 {
            return Err(ConsensusError::TransactionHasInvalidRing(
                "input does not have enough decoys",
            ));
        }
        if decoy_info.mixable > 1 {
            return Err(ConsensusError::TransactionHasInvalidInput(
                "more than one mixable input with unmixable inputs",
            ));
        }
    }

    if hf >= &HardFork::V8 && decoy_info.min_decoys != current_minimum_decoys {
        return Err(ConsensusError::TransactionHasInvalidRing(
            "one ring does not have the minimum number of decoys",
        ));
    }

    if hf >= &HardFork::V12 && decoy_info.min_decoys != decoy_info.max_decoys {
        return Err(ConsensusError::TransactionHasInvalidRing(
            "rings do not have the same number of members",
        ));
    }

    Ok(())
}

/// Checks the inputs key images for torsion and for duplicates in the transaction.
///
/// The `spent_kis` parameter is not meant to be a complete list of key images, just a list of related transactions
/// key images, for example transactions in a block. The chain will be checked for duplicates later.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#unique-key-image
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#torsion-free-key-image
pub(crate) fn check_key_images(
    input: &Input,
    spent_kis: &mut HashSet<[u8; 32]>,
) -> Result<(), ConsensusError> {
    match input {
        Input::ToKey { key_image, .. } => {
            // this happens in monero-serai but we may as well duplicate the check.
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

    Ok(())
}

/// Checks that the input is of type [`Input::ToKey`] aka txin_to_key.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#input-type
fn check_input_type(input: &Input) -> Result<(), ConsensusError> {
    match input {
        Input::ToKey { .. } => Ok(()),
        _ => Err(ConsensusError::TransactionHasInvalidInput(
            "Input not ToKey",
        )),
    }
}

/// Checks that the input has decoys.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#inputs-must-have-decoys
fn check_input_has_decoys(input: &Input) -> Result<(), ConsensusError> {
    match input {
        Input::ToKey { key_offsets, .. } => {
            if key_offsets.is_empty() {
                Err(ConsensusError::TransactionHasInvalidRing("No ring members"))
            } else {
                Ok(())
            }
        }
        _ => panic!("Input not ToKey"),
    }
}

/// Checks that the ring members for the input are unique after hard-fork 6.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#unique-ring-members
fn check_ring_members_unique(input: &Input, hf: &HardFork) -> Result<(), ConsensusError> {
    if hf >= &HardFork::V6 {
        match input {
            Input::ToKey { key_offsets, .. } => key_offsets.iter().skip(1).try_for_each(|offset| {
                if *offset == 0 {
                    Err(ConsensusError::TransactionHasInvalidRing(
                        "duplicate ring member",
                    ))
                } else {
                    Ok(())
                }
            }),
            _ => panic!("Only ToKey is allowed this should have already been checked!"),
        }
    } else {
        Ok(())
    }
}

/// Checks that from hf 7 the inputs are sorted by key image.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#sorted-inputs
fn check_inputs_sorted(inputs: &[Input], hf: &HardFork) -> Result<(), ConsensusError> {
    let get_ki = |inp: &Input| match inp {
        Input::ToKey { key_image, .. } => key_image.compress().to_bytes(),
        _ => panic!("Only ToKey is allowed this should have already been checked!"),
    };

    if hf >= &HardFork::V7 {
        for inps in inputs.windows(2) {
            match get_ki(&inps[0]).cmp(&get_ki(&inps[1])) {
                Ordering::Less => (),
                _ => {
                    return Err(ConsensusError::TransactionHasInvalidInput(
                        "Inputs not ordered by key image!",
                    ))
                }
            }
        }
        Ok(())
    } else {
        Ok(())
    }
}

/// Checks the youngest output is at least 10 blocks old.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#10-block-lock
fn check_10_block_lock(
    ring_member_info: &TxRingMembersInfo,
    current_chain_height: u64,
    hf: &HardFork,
) -> Result<(), ConsensusError> {
    if hf >= &HardFork::V12 {
        if ring_member_info.youngest_used_out_height + 10 > current_chain_height {
            Err(ConsensusError::TransactionHasInvalidRing(
                "tx has one ring member which is too young",
            ))
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

/// Sums the inputs checking for overflow.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#inputs-and-outputs-must-not-overflow
fn sum_inputs_v1(inputs: &[Input]) -> Result<u64, ConsensusError> {
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

/// Checks all input consensus rules.
///
/// TODO: list rules.
///
pub fn check_inputs(
    inputs: &[Input],
    ring_member_info: &TxRingMembersInfo,
    current_chain_height: u64,
    hf: &HardFork,
    tx_version: &TxVersion,
    spent_kis: Arc<std::sync::Mutex<HashSet<[u8; 32]>>>,
) -> Result<u64, ConsensusError> {
    if inputs.is_empty() {
        return Err(ConsensusError::TransactionHasInvalidInput("no inputs"));
    }

    check_10_block_lock(ring_member_info, current_chain_height, hf)?;

    if let Some(decoy_info) = &ring_member_info.decoy_info {
        check_decoy_info(decoy_info, hf)?;
    } else {
        assert_eq!(hf, &HardFork::V1);
    }

    for input in inputs {
        check_input_type(input)?;
        check_input_has_decoys(input)?;

        check_ring_members_unique(input, hf)?;

        let mut spent_kis_lock = spent_kis.lock().unwrap();
        check_key_images(input, &mut spent_kis_lock)?;
    }

    check_inputs_sorted(inputs, hf)?;

    match tx_version {
        TxVersion::RingSignatures => sum_inputs_v1(inputs),
        _ => panic!("TODO: RCT"),
    }
}
