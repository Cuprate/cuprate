use monero_serai::ringct::RctType;
use std::cmp::Ordering;

use monero_serai::transaction::{Input, Output, Timelock, Transaction};
use multiexp::BatchVerifier;

use crate::{
    blocks::penalty_free_zone, check_point_canonically_encoded, is_decomposed_amount, HardFork,
};

mod contextual_data;
mod ring_ct;
mod ring_signatures;

pub use contextual_data::*;
pub use ring_ct::RingCTError;

const MAX_BULLETPROOFS_OUTPUTS: usize = 16;
const MAX_TX_BLOB_SIZE: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TransactionError {
    #[error("The transactions version is incorrect.")]
    TransactionVersionInvalid,
    #[error("The transactions is too big.")]
    TooBig,
    //-------------------------------------------------------- OUTPUTS
    #[error("Output is not a valid point.")]
    OutputNotValidPoint,
    #[error("The transaction has an invalid output type.")]
    OutputTypeInvalid,
    #[error("The transaction is v1 with a 0 amount output.")]
    ZeroOutputForV1,
    #[error("The transaction is v2 with a non 0 amount output.")]
    NonZeroOutputForV2,
    #[error("The transaction has an output which is not decomposed.")]
    AmountNotDecomposed,
    #[error("The transactions outputs overflow.")]
    OutputsOverflow,
    #[error("The transactions outputs too much.")]
    OutputsTooHigh,
    #[error("The transactions has too many outputs.")]
    InvalidNumberOfOutputs,
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
    OneOrMoreRingMembersLocked,
    #[error("The transaction inputs overflow.")]
    InputsOverflow,
    #[error("The transaction has no inputs.")]
    NoInputs,
    #[error("Ring member not in database or is not valid.")]
    RingMemberNotFoundOrInvalid,
    //-------------------------------------------------------- Ring Signatures
    #[error("Ring signature incorrect.")]
    RingSignatureIncorrect,
    //-------------------------------------------------------- RingCT
    #[error("RingCT Error: {0}.")]
    RingCTError(#[from] RingCTError),
}

/// An enum representing all valid Monero transaction versions.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TxVersion {
    /// Legacy ring signatures.
    RingSignatures,
    /// RingCT
    RingCT,
}

impl TxVersion {
    /// Converts a `raw` version value to a [`TxVersion`].
    ///
    /// This will return `None` on invalid values.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/transactions.html#version>
    ///  &&  <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#version>
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
/// <https://monero-book.cuprate.org/consensus_rules/transactions/outputs.html#output-keys-canonical>
fn check_output_keys(outputs: &[Output]) -> Result<(), TransactionError> {
    for out in outputs {
        if !check_point_canonically_encoded(&out.key) {
            return Err(TransactionError::OutputNotValidPoint);
        }
    }

    Ok(())
}

/// Checks the output types are allowed for the given hard-fork.
///
/// This is also used during miner-tx verification.
///
/// <https://monero-book.cuprate.org/consensus_rules/transactions/outputs.html#output-type>
/// <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#output-type>
pub(crate) fn check_output_types(
    outputs: &[Output],
    hf: &HardFork,
) -> Result<(), TransactionError> {
    if hf == &HardFork::V15 {
        for outs in outputs.windows(2) {
            if outs[0].view_tag.is_some() != outs[1].view_tag.is_some() {
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

/// Checks the individual outputs amount for version 1 txs.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/outputs.html#output-amount>
fn check_output_amount_v1(amount: u64, hf: &HardFork) -> Result<(), TransactionError> {
    if amount == 0 {
        return Err(TransactionError::ZeroOutputForV1);
    }

    if hf >= &HardFork::V2 && !is_decomposed_amount(&amount) {
        return Err(TransactionError::AmountNotDecomposed);
    }

    Ok(())
}

/// Checks the individual outputs amount for version 2 txs.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/outputs.html#output-amount>
fn check_output_amount_v2(amount: u64) -> Result<(), TransactionError> {
    if amount == 0 {
        Ok(())
    } else {
        Err(TransactionError::NonZeroOutputForV2)
    }
}

/// Sums the outputs, checking for overflow and other consensus rules.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/outputs.html#output-amount>
/// &&   <https://monero-book.cuprate.org/consensus_rules/transactions/outputs.html#outputs-must-not-overflow>
fn sum_outputs(
    outputs: &[Output],
    hf: &HardFork,
    tx_version: &TxVersion,
) -> Result<u64, TransactionError> {
    let mut sum: u64 = 0;

    for out in outputs {
        let raw_amount = out.amount.unwrap_or(0);

        match tx_version {
            TxVersion::RingSignatures => check_output_amount_v1(raw_amount, hf)?,
            TxVersion::RingCT => check_output_amount_v2(raw_amount)?,
        }
        sum = sum
            .checked_add(raw_amount)
            .ok_or(TransactionError::OutputsOverflow)?;
    }

    Ok(sum)
}

/// Checks the number of outputs is allowed.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/outputs.html#2-outputs>
/// &&   <https://monero-book.cuprate.org/consensus_rules/transactions/ring_ct/bulletproofs.html#max-outputs>
/// &&   <https://monero-book.cuprate.org/consensus_rules/transactions/ring_ct/bulletproofs+.html#max-outputs>
fn check_number_of_outputs(
    outputs: usize,
    hf: &HardFork,
    tx_version: &TxVersion,
    rct_type: &RctType,
) -> Result<(), TransactionError> {
    if tx_version == &TxVersion::RingSignatures {
        return Ok(());
    }

    if hf >= &HardFork::V12 && outputs < 2 {
        return Err(TransactionError::InvalidNumberOfOutputs);
    }

    match rct_type {
        RctType::Bulletproofs
        | RctType::BulletproofsCompactAmount
        | RctType::Clsag
        | RctType::BulletproofsPlus => {
            if outputs <= MAX_BULLETPROOFS_OUTPUTS {
                Ok(())
            } else {
                Err(TransactionError::InvalidNumberOfOutputs)
            }
        }
        _ => Ok(()),
    }
}

/// Checks the outputs against all output consensus rules, returning the sum of the output amounts.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/outputs.html>
/// &&   <https://monero-book.cuprate.org/consensus_rules/transactions/ring_ct/bulletproofs.html#max-outputs>
/// &&   <https://monero-book.cuprate.org/consensus_rules/transactions/ring_ct/bulletproofs+.html#max-outputs>
fn check_outputs_semantics(
    outputs: &[Output],
    hf: &HardFork,
    tx_version: &TxVersion,
    rct_type: &RctType,
) -> Result<u64, TransactionError> {
    check_output_types(outputs, hf)?;
    check_output_keys(outputs)?;
    check_number_of_outputs(outputs.len(), hf, tx_version, rct_type)?;

    sum_outputs(outputs, hf, tx_version)
}

//----------------------------------------------------------------------------------------------------------- TIME LOCKS

/// Checks if an outputs unlock time has passed.
///
/// <https://monero-book.cuprate.org/consensus_rules/transactions/unlock_time.html>
pub fn output_unlocked(
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

/// Returns if a locked output, which uses a block height, can be spent.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/unlock_time.html#block-height>
fn check_block_time_lock(unlock_height: u64, current_chain_height: u64) -> bool {
    // current_chain_height = 1 + top height
    unlock_height <= current_chain_height
}

/// Returns if a locked output, which uses a block height, can be spend.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/unlock_time.html#timestamp>
fn check_timestamp_time_lock(
    unlock_timestamp: u64,
    current_time_lock_timestamp: u64,
    hf: &HardFork,
) -> bool {
    current_time_lock_timestamp + hf.block_time().as_secs() >= unlock_timestamp
}

/// Checks all the time locks are unlocked.
///
/// `current_time_lock_timestamp` must be: <https://monero-book.cuprate.org/consensus_rules/transactions/unlock_time.html#getting-the-current-time>
///
/// <https://monero-book.cuprate.org/consensus_rules/transactions/unlock_time.html>
/// <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#the-output-must-not-be-locked>
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
            tracing::debug!("Transaction invalid: one or more inputs locked, lock: {time_lock:?}.");
            Err(TransactionError::OneOrMoreRingMembersLocked)
        } else {
            Ok(())
        }
    })
}

//----------------------------------------------------------------------------------------------------------- INPUTS

/// Checks the decoys are allowed.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#minimum-decoys>
/// &&   <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#equal-number-of-decoys>
pub fn check_decoy_info(decoy_info: &DecoyInfo, hf: &HardFork) -> Result<(), TransactionError> {
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

/// Checks the inputs key images for torsion.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#torsion-free-key-image>
fn check_key_images(input: &Input) -> Result<(), TransactionError> {
    match input {
        Input::ToKey { key_image, .. } => {
            // this happens in monero-serai but we may as well duplicate the check.
            if !key_image.is_torsion_free() {
                return Err(TransactionError::KeyImageIsNotInPrimeSubGroup);
            }
        }
        _ => Err(TransactionError::IncorrectInputType)?,
    }

    Ok(())
}

/// Checks that the input is of type [`Input::ToKey`] aka txin_to_key.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#input-type>
fn check_input_type(input: &Input) -> Result<(), TransactionError> {
    match input {
        Input::ToKey { .. } => Ok(()),
        _ => Err(TransactionError::IncorrectInputType)?,
    }
}

/// Checks that the input has decoys.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#no-empty-decoys>
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
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#unique-ring-members>
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
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#sorted-inputs>
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
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#10-block-lock>
fn check_10_block_lock(
    youngest_used_out_height: u64,
    current_chain_height: u64,
    hf: &HardFork,
) -> Result<(), TransactionError> {
    if hf >= &HardFork::V12 {
        if youngest_used_out_height + 10 > current_chain_height {
            tracing::debug!(
                "Transaction invalid: One or more ring members younger than 10 blocks."
            );
            Err(TransactionError::OneOrMoreRingMembersLocked)
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

/// Sums the inputs checking for overflow.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#inputs-must-not-overflow>
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

/// Checks the inputs semantically validity, returning the sum of the inputs.
///
/// Semantic rules are rules that don't require blockchain context, the hard-fork does not require blockchain context as:
/// - The tx-pool will use the current hard-fork
/// - When syncing the hard-fork is in the block header.
fn check_inputs_semantics(inputs: &[Input], hf: &HardFork) -> Result<u64, TransactionError> {
    // <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#no-empty-inputs>
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

/// Checks the inputs contextual validity.
///
/// Contextual rules are rules that require blockchain context to check.
///
/// This function does not check signatures or for duplicate key-images.
fn check_inputs_contextual(
    inputs: &[Input],
    tx_ring_members_info: &TxRingMembersInfo,
    current_chain_height: u64,
    hf: &HardFork,
) -> Result<(), TransactionError> {
    // This rule is not contained in monero-core explicitly, but it is enforced by how Monero picks ring members.
    // When picking ring members monerod will only look in the DB at past blocks so an output has to be younger
    // than this transaction to be used in this tx.
    if tx_ring_members_info.youngest_used_out_height >= current_chain_height {
        tracing::debug!("Transaction invalid: One or more ring members too young.");
        Err(TransactionError::OneOrMoreRingMembersLocked)?;
    }

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

    for input in inputs {
        check_key_images(input)?;
    }

    Ok(())
}

//----------------------------------------------------------------------------------------------------------- OVERALL

/// Checks the version is in the allowed range.
///
/// <https://monero-book.cuprate.org/consensus_rules/transactions.html#version>
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

        let min = min_tx_version(hf);
        if version < &min && decoy_info.not_mixable == 0 {
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

/// Returns the default maximum tx version for the given hard-fork.
fn max_tx_version(hf: &HardFork) -> TxVersion {
    if hf <= &HardFork::V3 {
        TxVersion::RingSignatures
    } else {
        TxVersion::RingCT
    }
}

/// Returns the default minimum tx version for the given hard-fork.
fn min_tx_version(hf: &HardFork) -> TxVersion {
    if hf >= &HardFork::V6 {
        TxVersion::RingCT
    } else {
        TxVersion::RingSignatures
    }
}

fn transaction_weight_limit(hf: &HardFork) -> usize {
    penalty_free_zone(hf) / 2 - 600
}

/// Checks the transaction is semantically valid.
///
/// Semantic rules are rules that don't require blockchain context, the hard-fork does not require blockchain context as:
/// - The tx-pool will use the current hard-fork
/// - When syncing the hard-fork is in the block header.
///
/// To fully verify a transaction this must be accompanied with [`check_transaction_contextual`]
///
pub fn check_transaction_semantic(
    tx: &Transaction,
    tx_blob_size: usize,
    tx_weight: usize,
    tx_hash: &[u8; 32],
    hf: &HardFork,
    verifier: &mut BatchVerifier<(), dalek_ff_group::EdwardsPoint>,
) -> Result<u64, TransactionError> {
    // <https://monero-book.cuprate.org/consensus_rules/transactions.html#transaction-size>
    if tx_blob_size > MAX_TX_BLOB_SIZE
        || (hf >= &HardFork::V8 && tx_weight > transaction_weight_limit(hf))
    {
        Err(TransactionError::TooBig)?;
    }

    let tx_version = TxVersion::from_raw(tx.prefix.version)
        .ok_or(TransactionError::TransactionVersionInvalid)?;

    let outputs_sum = check_outputs_semantics(
        &tx.prefix.outputs,
        hf,
        &tx_version,
        &tx.rct_signatures.rct_type(),
    )?;
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

/// Checks the transaction is contextually valid.
///
/// To fully verify a transaction this must be accompanied by [`check_transaction_semantic`].
///
/// This function also does _not_ check for duplicate key-images: <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#unique-key-image>.
///
/// `current_time_lock_timestamp` must be: <https://monero-book.cuprate.org/consensus_rules/transactions/unlock_time.html#getting-the-current-time>.

pub fn check_transaction_contextual(
    tx: &Transaction,
    tx_ring_members_info: &TxRingMembersInfo,
    current_chain_height: u64,
    current_time_lock_timestamp: u64,
    hf: &HardFork,
) -> Result<(), TransactionError> {
    let tx_version = TxVersion::from_raw(tx.prefix.version)
        .ok_or(TransactionError::TransactionVersionInvalid)?;

    check_inputs_contextual(
        &tx.prefix.inputs,
        tx_ring_members_info,
        current_chain_height,
        hf,
    )?;
    check_tx_version(&tx_ring_members_info.decoy_info, &tx_version, hf)?;

    check_all_time_locks(
        &tx_ring_members_info.time_locked_outs,
        current_chain_height,
        current_time_lock_timestamp,
        hf,
    )?;

    match tx_version {
        TxVersion::RingSignatures => ring_signatures::check_input_signatures(
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

#[cfg(test)]
mod tests {
    use curve25519_dalek::EdwardsPoint;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use std::ops::Range;

    use crate::decomposed_amount::decomposed_amounts;
    use monero_serai::transaction::Output;

    use super::*;

    #[test]
    fn test_check_output_amount_v1() {
        for amount in decomposed_amounts() {
            assert!(check_output_amount_v1(*amount, &HardFork::V2).is_ok())
        }

        proptest!(|(amount in any::<u64>().prop_filter("value_decomposed", |val| !is_decomposed_amount(val)))| {
            prop_assert!(check_output_amount_v1(amount, &HardFork::V2).is_err());
            prop_assert!(check_output_amount_v1(amount, &HardFork::V1).is_ok())
        });
    }

    /// Returns a stratergy that resloves to a [`RctType`] that uses
    /// BPs(+).
    #[allow(unreachable_code)]
    #[allow(clippy::diverging_sub_expression)]
    fn bulletproof_rct_type() -> BoxedStrategy<RctType> {
        return prop_oneof![
            Just(RctType::Bulletproofs),
            Just(RctType::BulletproofsCompactAmount),
            Just(RctType::Clsag),
            Just(RctType::BulletproofsPlus),
        ]
        .boxed();

        // Here to make sure this is updated when needed.
        match unreachable!() {
            RctType::Null => {}
            RctType::MlsagAggregate => {}
            RctType::MlsagIndividual => {}
            RctType::Bulletproofs => {}
            RctType::BulletproofsCompactAmount => {}
            RctType::Clsag => {}
            RctType::BulletproofsPlus => {}
        };
    }

    prop_compose! {
        /// Returns a valid prime-order point.
        fn random_point()(bytes in any::<[u8; 32]>()) -> EdwardsPoint {
            EdwardsPoint::mul_base_clamped(bytes)
        }
    }

    prop_compose! {
        /// Returns a valid torsioned point.
        fn random_torsioned_point()(point in random_point(), torsion in 0..8_usize ) -> EdwardsPoint {
            point + curve25519_dalek::constants::EIGHT_TORSION[torsion]
        }
    }

    prop_compose! {
        /// Returns a random [`Output`].
        ///
        /// `key` is always valid.
        fn random_out(rct: bool, view_tagged: bool)(
            point in random_point(),
            amount in any::<u64>(),
            view_tag in any::<u8>(),
        ) -> Output {
            Output {
                amount: if rct { None } else { Some(amount) },
                key: point.compress(),
                view_tag: if view_tagged { Some(view_tag) } else { None },
            }
        }
    }

    prop_compose! {
        /// Returns a random [`Output`].
        ///
        /// `key` is always valid but torsioned.
        fn random_torsioned_out(rct: bool, view_tagged: bool)(
            point in random_torsioned_point(),
            amount in any::<u64>(),
            view_tag in any::<u8>(),
        ) -> Output {
            Output {
                amount: if rct { None } else { Some(amount) },
                key: point.compress(),
                view_tag: if view_tagged { Some(view_tag) } else { None },
            }
        }
    }

    prop_compose! {
        /// Returns a [`HardFork`] in a specific range.
        fn hf_in_range(range: Range<u8>)(
            hf in range,
        ) -> HardFork {
            HardFork::from_version(hf).unwrap()
        }
    }

    proptest! {
        #[test]
        fn test_check_output_keys(
            outs in vec(random_out(true, true), 0..16),
            torsioned_outs in vec(random_torsioned_out(false, true), 0..16)
        ) {
            prop_assert!(check_output_keys(&outs).is_ok());
            prop_assert!(check_output_keys(&torsioned_outs).is_ok());
        }

        #[test]
        fn output_types(
            mut view_tag_outs in vec(random_out(true, true), 1..16),
            mut non_view_tag_outs in vec(random_out(true, false), 1..16),
            hf_no_veiw_tags in hf_in_range(1..14),
            hf_veiw_tags in hf_in_range(16..17),
        ) {
            prop_assert!(check_output_types(&view_tag_outs, &hf_veiw_tags).is_ok());
            prop_assert!(check_output_types(&view_tag_outs, &hf_no_veiw_tags).is_err());


            prop_assert!(check_output_types(&non_view_tag_outs, &hf_no_veiw_tags).is_ok());
            prop_assert!(check_output_types(&non_view_tag_outs, &hf_veiw_tags).is_err());

            prop_assert!(check_output_types(&non_view_tag_outs, &HardFork::V15).is_ok());
            prop_assert!(check_output_types(&view_tag_outs, &HardFork::V15).is_ok());
            view_tag_outs.append(&mut non_view_tag_outs);
            prop_assert!(check_output_types(&view_tag_outs, &HardFork::V15).is_err());
        }

        #[test]
        fn test_valid_number_of_outputs(valid_numb_outs in 2..17_usize, rct_type in bulletproof_rct_type()) {
            prop_assert!(check_number_of_outputs(valid_numb_outs, &HardFork::V16, &TxVersion::RingCT, &rct_type).is_ok());
        }

        #[test]
        fn test_invalid_number_of_outputs(numb_outs in 17..160_000_usize, rct_type in bulletproof_rct_type()) {
            prop_assert!(check_number_of_outputs(numb_outs, &HardFork::V16, &TxVersion::RingCT, &rct_type).is_err());
        }
    }
}
