use std::cmp::max;

use monero_oxide::transaction::Timelock;
use thiserror::Error;

use cuprate_consensus_context::BlockchainContext;
use cuprate_consensus_rules::miner_tx::calculate_block_reward;
use cuprate_helper::cast::usize_to_u64;
use cuprate_types::TransactionVerificationData;

/// The maximum size of the tx extra field.
///
/// <https://github.com/monero-project/monero/blob/3b01c490953fe92f3c6628fa31d280a4f0490d28/src/cryptonote_config.h#L217>
const MAX_TX_EXTRA_SIZE: usize = 1060;

/// <https://github.com/monero-project/monero/blob/3b01c490953fe92f3c6628fa31d280a4f0490d28/src/cryptonote_config.h#L75>
const DYNAMIC_FEE_REFERENCE_TRANSACTION_WEIGHT: u128 = 3_000;

/// <https://github.com/monero-project/monero/blob/3b01c490953fe92f3c6628fa31d280a4f0490d28/src/cryptonote_core/blockchain.h#L646>
const FEE_MASK: u64 = 10_u64.pow(4);

#[derive(Debug, Error)]
pub enum RelayRuleError {
    #[error("Tx has non-zero timelock.")]
    NonZeroTimelock,
    #[error("Tx extra field is too large.")]
    ExtraFieldTooLarge,
    #[error("Tx fee too low.")]
    FeeBelowMinimum,
}

/// Checks the transaction passes the relay rules.
///
/// Relay rules are rules that govern the txs we accept to our tx-pool and propagate around the network.
pub fn check_tx_relay_rules(
    tx: &TransactionVerificationData,
    context: &BlockchainContext,
) -> Result<(), RelayRuleError> {
    if tx.tx.prefix().additional_timelock != Timelock::None {
        return Err(RelayRuleError::NonZeroTimelock);
    }

    if tx.tx.prefix().extra.len() > MAX_TX_EXTRA_SIZE {
        return Err(RelayRuleError::ExtraFieldTooLarge);
    }

    check_fee(tx.tx_weight, tx.fee, context)
}

/// Checks the fee is enough for the tx weight and current blockchain state.
fn check_fee(
    tx_weight: usize,
    fee: u64,
    context: &BlockchainContext,
) -> Result<(), RelayRuleError> {
    let base_reward = calculate_block_reward(
        1,
        context.effective_median_weight,
        context.already_generated_coins,
        context.current_hf,
    );

    let fee_per_byte = dynamic_base_fee(base_reward, context.effective_median_weight);
    let needed_fee = usize_to_u64(tx_weight) * fee_per_byte;

    let needed_fee = needed_fee.div_ceil(FEE_MASK) * FEE_MASK;

    if fee < (needed_fee - needed_fee / 50) {
        tracing::debug!(fee, needed_fee, "Tx fee is below minimum.");
        return Err(RelayRuleError::FeeBelowMinimum);
    }

    Ok(())
}

/// Calculates the base fee per byte for tx relay.
fn dynamic_base_fee(base_reward: u64, effective_media_block_weight: usize) -> u64 {
    let median_block_weight = effective_media_block_weight as u128;

    let fee_per_byte_100 = u128::from(base_reward) * DYNAMIC_FEE_REFERENCE_TRANSACTION_WEIGHT
        / median_block_weight
        / median_block_weight;
    let fee_per_byte = fee_per_byte_100 - fee_per_byte_100 / 20;

    #[expect(clippy::cast_possible_truncation)]
    max(fee_per_byte as u64, 1)
}
