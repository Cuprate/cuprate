use monero_serai::ringct::RctType;
use monero_serai::transaction::{Input, Output, Timelock, Transaction};

use crate::{
    transactions::{
        outputs::{check_output_types, is_decomposed_amount},
        TxVersion,
    },
    ConsensusError, HardFork,
};

const MONEY_SUPPLY: u64 = u64::MAX;
const MINIMUM_REWARD_PER_MIN: u64 = 3 * 10_u64.pow(11);

const MINER_TX_TIME_LOCKED_BLOCKS: u64 = 60;

fn calculate_base_reward(already_generated_coins: u64, hf: &HardFork) -> u64 {
    let target_mins = hf.block_time().as_secs() / 60;
    let emission_speed_factor = 20 - (target_mins - 1);
    ((MONEY_SUPPLY - already_generated_coins) >> emission_speed_factor)
        .max(MINIMUM_REWARD_PER_MIN * target_mins)
}

pub fn calculate_block_reward(
    block_weight: usize,
    median_bw: usize,
    already_generated_coins: u64,
    hf: &HardFork,
) -> u64 {
    tracing::info!("bw: {} median: {}", block_weight, median_bw);

    let base_reward: u128 = calculate_base_reward(already_generated_coins, hf).into();

    if block_weight <= median_bw {
        return base_reward.try_into().unwrap();
    }

    let multiplicand: u128 = ((2 * median_bw - block_weight) * block_weight)
        .try_into()
        .unwrap();
    let effective_median_bw: u128 = median_bw.try_into().unwrap();

    (((base_reward * multiplicand) / effective_median_bw) / effective_median_bw)
        .try_into()
        .unwrap()
}

/// Checks the miner transactions version.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks/miner_tx.html#version
fn check_tx_version(tx_version: &TxVersion, hf: &HardFork) -> Result<(), ConsensusError> {
    if hf >= &HardFork::V12 && tx_version != &TxVersion::RingCT {
        Err(ConsensusError::MinerTransaction("Version invalid"))
    } else {
        Ok(())
    }
}

/// Checks the miner transactions inputs.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks/miner_tx.html#input
/// https://cuprate.github.io/monero-book/consensus_rules/blocks/miner_tx.html#height
fn check_inputs(inputs: &[Input], chain_height: u64) -> Result<(), ConsensusError> {
    if inputs.len() != 1 {
        return Err(ConsensusError::MinerTransaction(
            "does not have exactly 1 input",
        ));
    }

    match &inputs[0] {
        Input::Gen(height) => {
            if height != &chain_height {
                Err(ConsensusError::MinerTransaction(
                    "Height in input is not expected height",
                ))
            } else {
                Ok(())
            }
        }
        _ => Err(ConsensusError::MinerTransaction("Input not of type Gen")),
    }
}

/// Checks the miner transaction has a correct time lock.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks/miner_tx.html#unlock-time
fn check_time_lock(time_lock: &Timelock, chain_height: u64) -> Result<(), ConsensusError> {
    match time_lock {
        Timelock::Block(till_height) => {
            if u64::try_from(*till_height).unwrap() != chain_height + MINER_TX_TIME_LOCKED_BLOCKS {
                Err(ConsensusError::MinerTransaction(
                    "Time lock has invalid block height",
                ))
            } else {
                Ok(())
            }
        }
        _ => Err(ConsensusError::MinerTransaction(
            "Time lock is not a block height",
        )),
    }
}

/// Sums the outputs checking for overflow.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks/miner_tx.html#output-amounts
fn sum_outputs(outputs: &[Output], hf: &HardFork) -> Result<u64, ConsensusError> {
    let mut sum: u64 = 0;
    for out in outputs {
        let amt = out.amount.unwrap_or(0);
        if hf == &HardFork::V3 && !is_decomposed_amount(amt) {
            return Err(ConsensusError::MinerTransaction(
                "output amount is not decomposed",
            ));
        }
        sum = sum
            .checked_add(amt)
            .ok_or(ConsensusError::MinerTransaction(
                "outputs overflow when summed",
            ))?;
    }
    Ok(sum)
}

/// Checks the total outputs amount is correct returning the amount of coins collected by the miner.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks/miner_tx.html#total-outputs
fn check_total_output_amt(
    total_output: u64,
    reward: u64,
    fees: u64,
    hf: &HardFork,
) -> Result<u64, ConsensusError> {
    if hf == &HardFork::V1 || hf >= &HardFork::V12 {
        if total_output != reward + fees {
            return Err(ConsensusError::MinerTransaction(
                "miner transaction does not output correct amt",
            ));
        }
        Ok(reward)
    } else {
        if total_output - fees > reward {
            return Err(ConsensusError::MinerTransaction(
                "miner transaction does not output correct amt",
            ));
        }

        if total_output > reward + fees {
            return Err(ConsensusError::MinerTransaction(
                "miner transaction does not output correct amt",
            ));
        }
        Ok(total_output - fees)
    }
}

pub fn check_miner_tx(
    tx: &Transaction,
    total_fees: u64,
    chain_height: u64,
    block_weight: usize,
    median_bw: usize,
    already_generated_coins: u64,
    hf: &HardFork,
) -> Result<u64, ConsensusError> {
    let tx_version = TxVersion::from_raw(tx.prefix.version)?;
    check_tx_version(&tx_version, hf)?;

    if hf >= &HardFork::V12 && tx.rct_signatures.rct_type() != RctType::Null {
        return Err(ConsensusError::MinerTransaction("RctType is not null"));
    }

    check_time_lock(&tx.prefix.timelock, chain_height)?;

    check_inputs(&tx.prefix.inputs, chain_height)?;

    check_output_types(&tx.prefix.outputs, hf)?;

    let reward = calculate_block_reward(block_weight, median_bw, already_generated_coins, hf);
    let total_outs = sum_outputs(&tx.prefix.outputs, hf)?;

    check_total_output_amt(total_outs, reward, total_fees, hf)
}
