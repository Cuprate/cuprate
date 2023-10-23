use std::sync::OnceLock;

use monero_serai::transaction::Output;

use crate::{helper::check_point, transactions::TxVersion, ConsensusError, HardFork};

/// Decomposed amount table.
///
/// TODO: manually list each amount
static DECOMPOSED_AMOUNTS: OnceLock<[u64; 172]> = OnceLock::new();

pub(crate) fn decomposed_amounts() -> &'static [u64; 172] {
    DECOMPOSED_AMOUNTS.get_or_init(|| {
        let mut amounts = [1; 172];
        for zeros in 0..19 {
            for digit in 1..10 {
                amounts[zeros * 9 + digit - 1] =
                    (digit * 10_usize.pow(zeros as u32)).try_into().unwrap();
            }
        }
        amounts[171] = 10000000000000000000;
        amounts
    })
}

/// Checks the output keys are canonical points.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#output-keys-canonical
fn check_output_keys(outputs: &[Output]) -> Result<(), ConsensusError> {
    for out in outputs {
        if !check_point(&out.key) {
            return Err(ConsensusError::TransactionInvalidOutput(
                "outputs one time key is not a valid point",
            ));
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
pub(crate) fn check_output_types(outputs: &[Output], hf: &HardFork) -> Result<(), ConsensusError> {
    if hf == &HardFork::V15 {
        for outs in outputs.windows(2) {
            if outs[0].view_tag.is_some() != outs[0].view_tag.is_some() {
                return Err(ConsensusError::TransactionInvalidOutput(
                    "v15 output must have one output type",
                ));
            }
        }
        return Ok(());
    }

    for out in outputs {
        if hf <= &HardFork::V14 && out.view_tag.is_some() {
            return Err(ConsensusError::TransactionInvalidOutput(
                "tagged output used before allowed",
            ));
        } else if hf >= &HardFork::V16 && out.view_tag.is_none() {
            return Err(ConsensusError::TransactionInvalidOutput(
                "output does not have a view tag",
            ));
        }
    }
    Ok(())
}

/// Checks that an output amount is decomposed.
///
/// This is also used during miner tx verification.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#output-amount
/// https://cuprate.github.io/monero-book/consensus_rules/blocks/miner_tx.html#output-amounts
pub(crate) fn is_decomposed_amount(amount: u64) -> bool {
    decomposed_amounts().binary_search(&amount).is_ok()
}

/// Checks the outputs amount for version 1 txs.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#output-amount
fn check_output_amount_v1(amount: u64, hf: &HardFork) -> Result<(), ConsensusError> {
    if amount == 0 {
        return Err(ConsensusError::TransactionInvalidOutput(
            "zero amount output for v1 tx",
        ));
    }

    if hf >= &HardFork::V2 && !is_decomposed_amount(amount) {
        return Err(ConsensusError::TransactionInvalidOutput(
            "v1 tx does not have decomposed amount",
        ));
    }

    Ok(())
}

/// Sums the outputs, checking for overflow and other consensus rules.
///
/// Should only be used on v1 transactions.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#inputs-and-outputs-must-not-overflow
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#output-amount
fn sum_outputs_v1(outputs: &[Output], hf: &HardFork) -> Result<u64, ConsensusError> {
    let mut sum: u64 = 0;

    for out in outputs {
        let raw_amount = out.amount.unwrap_or(0);

        check_output_amount_v1(raw_amount, hf)?;

        sum = sum
            .checked_add(raw_amount)
            .ok_or(ConsensusError::TransactionOutputsOverflow)?;
    }

    Ok(sum)
}

/// Checks the outputs against all output consensus rules, returning the sum of the output amounts.
pub fn check_outputs(
    outputs: &[Output],
    hf: &HardFork,
    tx_version: &TxVersion,
) -> Result<u64, ConsensusError> {
    check_output_types(outputs, &hf)?;
    check_output_keys(outputs)?;

    match tx_version {
        TxVersion::RingSignatures => sum_outputs_v1(outputs, &hf),
        _ => todo!("RingCT"),
    }
}
