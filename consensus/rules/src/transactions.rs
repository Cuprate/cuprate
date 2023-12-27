use std::{cmp::Ordering, collections::HashSet, sync::Arc};

use monero_serai::transaction::{Input, Output, Timelock, Transaction};
use multiexp::BatchVerifier;

use crate::{check_point_canonically_encoded, is_decomposed_amount, HardFork};

mod contextual_data;
mod ring_ct;
mod ring_signatures;

pub use contextual_data::*;
pub use ring_ct::RingCTError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TransactionError {
    #[error("The transactions version is incorrect.")]
    TransactionVersionInvalid,
    //-------------------------------------------------------- OUTPUTS
    #[error("Output is not a valid point.")]
    OutputNotValidPoint,
    #[error("The transaction has an invalid output type.")]
    OutputTypeInvalid,
    #[error("The transaction is v1 with a 0 amount output.")]
    ZeroOutputForV1,
    #[error("The transaction has an output which is not decomposed.")]
    AmountNotDecomposed,
    #[error("The transactions outputs overflow.")]
    OutputsOverflow,
    #[error("The transactions outputs too much.")]
    OutputsTooHigh,
    //-------------------------------------------------------- INPUTS
    #[error("One or more inputs don't have the expected number of decoys.")]
    InputDoesNotHaveExpectedNumbDecoys,
    #[error("The transaction has more than one mixable input with unmixable inputs.")]
    MoreThanOneMixableInputWithUnmixable,
    #[error("The key-image is not in the prime sub-group.")]
    KeyImageIsNotInPrimeSubGroup,
    #[error("Key-image is already spent.")]
    KeyImageSpent,
    #[error("The input is not the expected type.")]
    IncorrectInputType,
    #[error("The transaction has a duplicate ring member.")]
    DuplicateRingMember,
    #[error("The transaction inputs are not ordered.")]
    InputsAreNotOrdered,
    #[error("The transaction spends a decoy which is too young.")]
    OneOrMoreDecoysLocked,
    #[error("The transaction inputs overflow.")]
    InputsOverflow,
    #[error("The transaction has no inputs.")]
    NoInputs,
    #[error("Ring member not in database")]
    RingMemberNotFound,
    //-------------------------------------------------------- Ring Signatures
    #[error("Ring signature incorrect.")]
    RingSignatureIncorrect,
    //-------------------------------------------------------- RingCT
    #[error("RingCT Error: {0}.")]
    RingCTError(#[from] ring_ct::RingCTError),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TxVersion {
    RingSignatures,
    RingCT,
}

impl TxVersion {
    pub fn from_raw(version: u64) -> Option<TxVersion> {
        Some(match version {
            1 => TxVersion::RingSignatures,
            2 => TxVersion::RingCT,
            _ => return None,
        })
    }
}

//----------------------------------------------------------------------------------------------------------- OUTPUTS

/// Checks the output keys are canonically encoded points.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#output-keys-canonical
fn check_output_keys(outputs: &[Output]) -> Result<(), TransactionError> {
    for out in outputs {
        if !check_point_canonically_encoded(&out.key) {
            return Err(TransactionError::OutputNotValidPoint);
        }
    }

    Ok(())
}

/// Checks the output types are allowed.
///
/// This is also used during miner-tx verification.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#output-type
/// https://cuprate.github.io/monero-book/consensus_rules/blocks/miner_tx.html#output-type
pub(crate) fn check_output_types(
    outputs: &[Output],
    hf: &HardFork,
) -> Result<(), TransactionError> {
    if hf == &HardFork::V15 {
        for outs in outputs.windows(2) {
            if outs[0].view_tag.is_some() != outs[0].view_tag.is_some() {
                return Err(TransactionError::OutputTypeInvalid);
            }
        }
        return Ok(());
    }

    for out in outputs {
        if hf <= &HardFork::V14 && out.view_tag.is_some()
            || hf >= &HardFork::V16 && out.view_tag.is_none()
        {
            return Err(TransactionError::OutputTypeInvalid);
        }
    }
    Ok(())
}

/// Checks the outputs amount for version 1 txs.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#output-amount
fn check_output_amount_v1(amount: u64, hf: &HardFork) -> Result<(), TransactionError> {
    if amount == 0 {
        return Err(TransactionError::ZeroOutputForV1);
    }

    if hf >= &HardFork::V2 && !is_decomposed_amount(&amount) {
        return Err(TransactionError::AmountNotDecomposed);
    }

    Ok(())
}

/// Sums the outputs, checking for overflow and other consensus rules.
///
/// Should only be used on v1 transactions.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#inputs-and-outputs-must-not-overflow
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#output-amount
fn sum_outputs_v1(outputs: &[Output], hf: &HardFork) -> Result<u64, TransactionError> {
    let mut sum: u64 = 0;

    for out in outputs {
        let raw_amount = out.amount.unwrap_or(0);

        check_output_amount_v1(raw_amount, hf)?;

        sum = sum
            .checked_add(raw_amount)
            .ok_or(TransactionError::OutputsOverflow)?;
    }

    Ok(sum)
}

/// Checks the outputs against all output consensus rules, returning the sum of the output amounts.
fn check_outputs_semantics(
    outputs: &[Output],
    hf: &HardFork,
    tx_version: &TxVersion,
) -> Result<u64, TransactionError> {
    check_output_types(outputs, hf)?;
    check_output_keys(outputs)?;

    match tx_version {
        TxVersion::RingSignatures => sum_outputs_v1(outputs, hf),
        TxVersion::RingCT => Ok(0), // RCT outputs are checked to be zero in RCT checks.
    }
}

//----------------------------------------------------------------------------------------------------------- TIME LOCKS

/// Checks if an outputs unlock time has passed.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#unlock-time
fn output_unlocked(
    time_lock: &Timelock,
    current_chain_height: u64,
    current_time_lock_timestamp: u64,
    hf: &HardFork,
) -> bool {
    match *time_lock {
        Timelock::None => true,
        Timelock::Block(unlock_height) => {
            check_block_time_lock(unlock_height.try_into().unwrap(), current_chain_height)
        }
        Timelock::Time(unlock_time) => {
            check_timestamp_time_lock(unlock_time, current_time_lock_timestamp, hf)
        }
    }
}

/// Returns if a locked output, which uses a block height, can be spend.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#block-height
fn check_block_time_lock(unlock_height: u64, current_chain_height: u64) -> bool {
    // current_chain_height = 1 + top height
    unlock_height <= current_chain_height
}

/// ///
/// Returns if a locked output, which uses a block height, can be spend.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#timestamp
fn check_timestamp_time_lock(
    unlock_timestamp: u64,
    current_time_lock_timestamp: u64,
    hf: &HardFork,
) -> bool {
    current_time_lock_timestamp + hf.block_time().as_secs() >= unlock_timestamp
}

/// Checks all the time locks are unlocked.
///
/// `current_time_lock_timestamp` must be: https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#getting-the-current-time
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#unlock-time
fn check_all_time_locks(
    time_locks: &[Timelock],
    current_chain_height: u64,
    current_time_lock_timestamp: u64,
    hf: &HardFork,
) -> Result<(), TransactionError> {
    time_locks.iter().try_for_each(|time_lock| {
        if !output_unlocked(
            time_lock,
            current_chain_height,
            current_time_lock_timestamp,
            hf,
        ) {
            Err(TransactionError::OneOrMoreDecoysLocked)
        } else {
            Ok(())
        }
    })
}

//----------------------------------------------------------------------------------------------------------- INPUTS

/// Checks the decoys are allowed.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#minimum-decoys
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#equal-number-of-decoys
fn check_decoy_info(decoy_info: &DecoyInfo, hf: &HardFork) -> Result<(), TransactionError> {
    if hf == &HardFork::V15 {
        // Hard-fork 15 allows both v14 and v16 rules
        return check_decoy_info(decoy_info, &HardFork::V14)
            .or_else(|_| check_decoy_info(decoy_info, &HardFork::V16));
    }

    let current_minimum_decoys = minimum_decoys(hf);

    if decoy_info.min_decoys < current_minimum_decoys {
        // Only allow rings without enough decoys if there aren't enough decoys to mix with.
        if decoy_info.not_mixable == 0 {
            return Err(TransactionError::InputDoesNotHaveExpectedNumbDecoys);
        }
        // Only allow upto 1 mixable input with unmixable inputs.
        if decoy_info.mixable > 1 {
            return Err(TransactionError::MoreThanOneMixableInputWithUnmixable);
        }
    } else if hf >= &HardFork::V8 && decoy_info.min_decoys != current_minimum_decoys {
        // From V8 enforce the minimum used number of rings is the default minimum.
        return Err(TransactionError::InputDoesNotHaveExpectedNumbDecoys);
    }

    // From v12 all inputs must have the same number of decoys.
    if hf >= &HardFork::V12 && decoy_info.min_decoys != decoy_info.max_decoys {
        return Err(TransactionError::InputDoesNotHaveExpectedNumbDecoys);
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
fn check_key_images(
    input: &Input,
    spent_kis: &mut HashSet<[u8; 32]>,
) -> Result<(), TransactionError> {
    match input {
        Input::ToKey { key_image, .. } => {
            // this happens in monero-serai but we may as well duplicate the check.
            if !key_image.is_torsion_free() {
                return Err(TransactionError::KeyImageIsNotInPrimeSubGroup);
            }
            if !spent_kis.insert(key_image.compress().to_bytes()) {
                return Err(TransactionError::KeyImageSpent);
            }
        }
        _ => Err(TransactionError::IncorrectInputType)?,
    }

    Ok(())
}

/// Checks that the input is of type [`Input::ToKey`] aka txin_to_key.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#input-type
fn check_input_type(input: &Input) -> Result<(), TransactionError> {
    match input {
        Input::ToKey { .. } => Ok(()),
        _ => Err(TransactionError::IncorrectInputType)?,
    }
}

/// Checks that the input has decoys.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#inputs-must-have-decoys
fn check_input_has_decoys(input: &Input) -> Result<(), TransactionError> {
    match input {
        Input::ToKey { key_offsets, .. } => {
            if key_offsets.is_empty() {
                Err(TransactionError::InputDoesNotHaveExpectedNumbDecoys)
            } else {
                Ok(())
            }
        }
        _ => Err(TransactionError::IncorrectInputType)?,
    }
}

/// Checks that the ring members for the input are unique after hard-fork 6.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#unique-ring-members
fn check_ring_members_unique(input: &Input, hf: &HardFork) -> Result<(), TransactionError> {
    if hf >= &HardFork::V6 {
        match input {
            Input::ToKey { key_offsets, .. } => key_offsets.iter().skip(1).try_for_each(|offset| {
                if *offset == 0 {
                    Err(TransactionError::DuplicateRingMember)
                } else {
                    Ok(())
                }
            }),
            _ => Err(TransactionError::IncorrectInputType)?,
        }
    } else {
        Ok(())
    }
}

/// Checks that from hf 7 the inputs are sorted by key image.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#sorted-inputs
fn check_inputs_sorted(inputs: &[Input], hf: &HardFork) -> Result<(), TransactionError> {
    let get_ki = |inp: &Input| match inp {
        Input::ToKey { key_image, .. } => Ok(key_image.compress().to_bytes()),
        _ => Err(TransactionError::IncorrectInputType),
    };

    if hf >= &HardFork::V7 {
        for inps in inputs.windows(2) {
            match get_ki(&inps[0])?.cmp(&get_ki(&inps[1])?) {
                Ordering::Greater => (),
                _ => return Err(TransactionError::InputsAreNotOrdered),
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
    youngest_used_out_height: u64,
    current_chain_height: u64,
    hf: &HardFork,
) -> Result<(), TransactionError> {
    if hf >= &HardFork::V12 {
        if youngest_used_out_height + 10 > current_chain_height {
            Err(TransactionError::OneOrMoreDecoysLocked)
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
fn sum_inputs_check_overflow(inputs: &[Input]) -> Result<u64, TransactionError> {
    let mut sum: u64 = 0;
    for inp in inputs {
        match inp {
            Input::ToKey { amount, .. } => {
                sum = sum
                    .checked_add(amount.unwrap_or(0))
                    .ok_or(TransactionError::InputsOverflow)?;
            }
            _ => Err(TransactionError::IncorrectInputType)?,
        }
    }

    Ok(sum)
}

fn check_inputs_semantics(inputs: &[Input], hf: &HardFork) -> Result<u64, TransactionError> {
    if inputs.is_empty() {
        return Err(TransactionError::NoInputs);
    }

    for input in inputs {
        check_input_type(input)?;
        check_input_has_decoys(input)?;

        check_ring_members_unique(input, hf)?;
    }

    check_inputs_sorted(inputs, hf)?;

    sum_inputs_check_overflow(inputs)
}

fn check_inputs_contextual(
    inputs: &[Input],
    tx_ring_members_info: &TxRingMembersInfo,
    current_chain_height: u64,
    hf: &HardFork,
    spent_kis: Arc<std::sync::Mutex<HashSet<[u8; 32]>>>,
) -> Result<(), TransactionError> {
    check_10_block_lock(
        tx_ring_members_info.youngest_used_out_height,
        current_chain_height,
        hf,
    )?;

    if let Some(decoys_info) = &tx_ring_members_info.decoy_info {
        check_decoy_info(decoys_info, hf)?;
    } else {
        assert_eq!(hf, &HardFork::V1);
    }

    let mut spent_kis_lock = spent_kis.lock().unwrap();
    for input in inputs {
        check_key_images(input, &mut spent_kis_lock)?;
    }
    drop(spent_kis_lock);

    Ok(())
}

/// Checks the version is in the allowed range.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#version
fn check_tx_version(
    decoy_info: &Option<DecoyInfo>,
    version: &TxVersion,
    hf: &HardFork,
) -> Result<(), TransactionError> {
    if let Some(decoy_info) = decoy_info {
        let max = max_tx_version(hf);
        if version > &max {
            return Err(TransactionError::TransactionVersionInvalid);
        }

        // TODO: Doc is wrong here
        let min = min_tx_version(hf);
        if version < &min && decoy_info.not_mixable != 0 {
            return Err(TransactionError::TransactionVersionInvalid);
        }
    } else {
        // This will only happen for hard-fork 1 when only RingSignatures are allowed.
        if version != &TxVersion::RingSignatures {
            return Err(TransactionError::TransactionVersionInvalid);
        }
    }

    Ok(())
}

fn max_tx_version(hf: &HardFork) -> TxVersion {
    if hf <= &HardFork::V3 {
        TxVersion::RingSignatures
    } else {
        TxVersion::RingCT
    }
}

fn min_tx_version(hf: &HardFork) -> TxVersion {
    if hf >= &HardFork::V6 {
        TxVersion::RingCT
    } else {
        TxVersion::RingSignatures
    }
}

pub fn check_transaction_semantic(
    tx: &Transaction,
    tx_hash: &[u8; 32],
    hf: &HardFork,
    verifier: &mut BatchVerifier<(), dalek_ff_group::EdwardsPoint>,
) -> Result<u64, TransactionError> {
    let tx_version = TxVersion::from_raw(tx.prefix.version)
        .ok_or(TransactionError::TransactionVersionInvalid)?;

    let outputs_sum = check_outputs_semantics(&tx.prefix.outputs, hf, &tx_version)?;
    let inputs_sum = check_inputs_semantics(&tx.prefix.inputs, hf)?;

    let fee = match tx_version {
        TxVersion::RingSignatures => {
            if outputs_sum >= inputs_sum {
                Err(TransactionError::OutputsTooHigh)?;
            }
            inputs_sum - outputs_sum
        }
        TxVersion::RingCT => {
            ring_ct::ring_ct_semantic_checks(tx, tx_hash, verifier, hf)?;

            tx.rct_signatures.base.fee
        }
    };

    Ok(fee)
}

pub fn check_transaction_contextual(
    tx: &Transaction,
    tx_ring_members_info: &TxRingMembersInfo,
    current_chain_height: u64,
    current_time_lock_timestamp: u64,
    hf: &HardFork,
    spent_kis: Arc<std::sync::Mutex<HashSet<[u8; 32]>>>,
) -> Result<(), TransactionError> {
    let tx_version = TxVersion::from_raw(tx.prefix.version)
        .ok_or(TransactionError::TransactionVersionInvalid)?;

    check_inputs_contextual(
        &tx.prefix.inputs,
        tx_ring_members_info,
        current_chain_height,
        hf,
        spent_kis,
    )?;
    check_tx_version(&tx_ring_members_info.decoy_info, &tx_version, hf)?;

    check_all_time_locks(
        &tx_ring_members_info.time_locked_outs,
        current_chain_height,
        current_time_lock_timestamp,
        hf,
    )?;

    match tx_version {
        TxVersion::RingSignatures => ring_signatures::verify_inputs_signatures(
            &tx.prefix.inputs,
            &tx.signatures,
            &tx_ring_members_info.rings,
            &tx.signature_hash(),
        ),
        TxVersion::RingCT => Ok(ring_ct::check_input_signatures(
            &tx.signature_hash(),
            &tx.prefix.inputs,
            &tx.rct_signatures,
            &tx_ring_members_info.rings,
        )?),
    }
}
