use monero_serai::transaction::{Input, Output, Timelock, Transaction};

use cuprate_constants::block::MAX_BLOCK_HEIGHT_USIZE;
use cuprate_types::TxVersion;

use crate::{is_decomposed_amount, transactions::check_output_types, HardFork};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MinerTxError {
    #[error("The miners transaction version is invalid.")]
    VersionInvalid,
    #[error("The miner transaction does not have exactly one input.")]
    IncorrectNumbOfInputs,
    #[error("The miner transactions input has the wrong block height.")]
    InputsHeightIncorrect,
    #[error("The input is not of type `gen`.")]
    InputNotOfTypeGen,
    #[error("The transaction has an incorrect lock time.")]
    InvalidLockTime,
    #[error("The transaction has an output which is not decomposed.")]
    OutputNotDecomposed,
    #[error("The transaction outputs overflow when summed.")]
    OutputsOverflow,
    #[error("The miner transaction outputs the wrong amount.")]
    OutputAmountIncorrect,
    #[error("The miner transactions RCT type is not NULL.")]
    RCTTypeNotNULL,
    #[error("The miner transactions has an invalid output type.")]
    InvalidOutputType,
}

/// A constant called "money supply", not actually a cap, it is used during
/// block reward calculations.
const MONEY_SUPPLY: u64 = u64::MAX;
/// The minimum block reward per minute, "tail-emission"
const MINIMUM_REWARD_PER_MIN: u64 = 3 * 10_u64.pow(11);
/// The value which `lock_time` should be for a coinbase output.
const MINER_TX_TIME_LOCKED_BLOCKS: usize = 60;

/// Calculates the base block reward without taking away the penalty for expanding
/// the block.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/reward.html#calculating-base-block-reward>
fn calculate_base_reward(already_generated_coins: u64, hf: &HardFork) -> u64 {
    let target_mins = hf.block_time().as_secs() / 60;
    let emission_speed_factor = 20 - (target_mins - 1);
    ((MONEY_SUPPLY - already_generated_coins) >> emission_speed_factor)
        .max(MINIMUM_REWARD_PER_MIN * target_mins)
}

/// Calculates the miner reward for this block.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/reward.html#calculating-block-reward>
pub fn calculate_block_reward(
    block_weight: usize,
    median_bw: usize,
    already_generated_coins: u64,
    hf: &HardFork,
) -> u64 {
    let base_reward = calculate_base_reward(already_generated_coins, hf);

    if block_weight <= median_bw {
        return base_reward;
    }

    let multiplicand: u128 = ((2 * median_bw - block_weight) * block_weight)
        .try_into()
        .unwrap();
    let effective_median_bw: u128 = median_bw.try_into().unwrap();

    (((base_reward as u128 * multiplicand) / effective_median_bw) / effective_median_bw)
        .try_into()
        .unwrap()
}

/// Checks the miner transactions version.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#version>
fn check_miner_tx_version(tx_version: &TxVersion, hf: &HardFork) -> Result<(), MinerTxError> {
    // The TxVersion enum checks if the version is not 1 or 2
    if hf >= &HardFork::V12 && tx_version != &TxVersion::RingCT {
        Err(MinerTxError::VersionInvalid)
    } else {
        Ok(())
    }
}

/// Checks the miner transactions inputs.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#input>
fn check_inputs(inputs: &[Input], chain_height: usize) -> Result<(), MinerTxError> {
    if inputs.len() != 1 {
        return Err(MinerTxError::IncorrectNumbOfInputs);
    }

    match &inputs[0] {
        Input::Gen(height) => {
            if height != &chain_height {
                Err(MinerTxError::InputsHeightIncorrect)
            } else {
                Ok(())
            }
        }
        _ => Err(MinerTxError::InputNotOfTypeGen),
    }
}

/// Checks the miner transaction has a correct time lock.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#unlock-time>
fn check_time_lock(time_lock: &Timelock, chain_height: usize) -> Result<(), MinerTxError> {
    match time_lock {
        &Timelock::Block(till_height) => {
            // Lock times above this amount are timestamps not blocks.
            // This is just for safety though and shouldn't actually be hit.
            if till_height > MAX_BLOCK_HEIGHT_USIZE {
                Err(MinerTxError::InvalidLockTime)?;
            }
            if till_height != chain_height + MINER_TX_TIME_LOCKED_BLOCKS {
                Err(MinerTxError::InvalidLockTime)
            } else {
                Ok(())
            }
        }
        _ => Err(MinerTxError::InvalidLockTime),
    }
}

/// Sums the outputs checking for overflow.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#output-amounts>
/// &&   <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#zero-amount-v1-output>
fn sum_outputs(
    outputs: &[Output],
    hf: &HardFork,
    tx_version: &TxVersion,
) -> Result<u64, MinerTxError> {
    let mut sum: u64 = 0;
    for out in outputs {
        let amt = out.amount.unwrap_or(0);

        if tx_version == &TxVersion::RingSignatures && amt == 0 {
            return Err(MinerTxError::OutputAmountIncorrect);
        }

        if hf == &HardFork::V3 && !is_decomposed_amount(&amt) {
            return Err(MinerTxError::OutputNotDecomposed);
        }
        sum = sum.checked_add(amt).ok_or(MinerTxError::OutputsOverflow)?;
    }
    Ok(sum)
}

/// Checks the total outputs amount is correct returning the amount of coins collected by the miner.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#total-outputs>
fn check_total_output_amt(
    total_output: u64,
    reward: u64,
    fees: u64,
    hf: &HardFork,
) -> Result<u64, MinerTxError> {
    if hf == &HardFork::V1 || hf >= &HardFork::V12 {
        if total_output != reward + fees {
            return Err(MinerTxError::OutputAmountIncorrect);
        }
        Ok(reward)
    } else {
        if total_output - fees > reward || total_output > reward + fees {
            return Err(MinerTxError::OutputAmountIncorrect);
        }
        Ok(total_output - fees)
    }
}

/// Checks all miner transactions rules.
///
/// Excluding:
/// <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#v2-output-pool>
///
/// as this needs to be done in a database.
pub fn check_miner_tx(
    tx: &Transaction,
    total_fees: u64,
    chain_height: usize,
    block_weight: usize,
    median_bw: usize,
    already_generated_coins: u64,
    hf: &HardFork,
) -> Result<u64, MinerTxError> {
    let tx_version = TxVersion::from_raw(tx.version()).ok_or(MinerTxError::VersionInvalid)?;
    check_miner_tx_version(&tx_version, hf)?;

    // ref: <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#ringct-type>
    match tx {
        Transaction::V1 { .. } => (),
        Transaction::V2 { proofs, .. } => {
            if hf >= &HardFork::V12 && proofs.is_some() {
                return Err(MinerTxError::RCTTypeNotNULL);
            }
        }
    }

    check_time_lock(&tx.prefix().additional_timelock, chain_height)?;

    check_inputs(&tx.prefix().inputs, chain_height)?;

    check_output_types(&tx.prefix().outputs, hf).map_err(|_| MinerTxError::InvalidOutputType)?;

    let reward = calculate_block_reward(block_weight, median_bw, already_generated_coins, hf);
    let total_outs = sum_outputs(&tx.prefix().outputs, hf, &tx_version)?;

    check_total_output_amt(total_outs, reward, total_fees, hf)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn tail_emission(generated_coins in any::<u64>(), hf in any::<HardFork>()) {
            prop_assert!(calculate_base_reward(generated_coins, &hf) >= MINIMUM_REWARD_PER_MIN * hf.block_time().as_secs() / 60)
        }
    }
}
