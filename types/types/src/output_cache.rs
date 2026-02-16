use cuprate_helper::{cast::u64_to_usize, crypto::compute_zero_commitment};
use indexmap::{IndexMap, IndexSet};
use monero_oxide::transaction::Pruned;
use monero_oxide::{io::CompressedPoint, transaction::Transaction};

use crate::{OutputOnChain, VerifiedBlockInformation};

/// A cache of outputs from the blockchain database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputCache {
    /// A map of (amount, amount idx) -> output.
    cached_outputs: IndexMap<u64, IndexMap<u64, OutputOnChain>>,
    /// A map of an output amount to the amount of outputs in the blockchain with that amount.
    number_of_outputs: IndexMap<u64, u64>,
    /// A set of outputs that were requested but were not currently in the DB.
    wanted_outputs: IndexMap<u64, IndexSet<u64>>,
}

impl OutputCache {
    /// Create a new [`OutputCache`].
    pub const fn new(
        cached_outputs: IndexMap<u64, IndexMap<u64, OutputOnChain>>,
        number_of_outputs: IndexMap<u64, u64>,
        wanted_outputs: IndexMap<u64, IndexSet<u64>>,
    ) -> Self {
        Self {
            cached_outputs,
            number_of_outputs,
            wanted_outputs,
        }
    }

    /// Returns the set of currently cached outputs.
    ///
    /// # Warning
    ///
    /// [`Self::get_output`] should be preferred over this when possible, this will not contain all outputs
    /// asked for necessarily.
    pub const fn cached_outputs(&self) -> &IndexMap<u64, IndexMap<u64, OutputOnChain>> {
        &self.cached_outputs
    }

    /// Returns the number of outputs in the blockchain with the given amount.
    ///
    /// # Warning
    ///
    /// The cache will only track the amount of outputs with a given amount for the requested outputs.
    /// So if you do not request an output with `amount` when generating the cache the amount of outputs
    /// with value `amount` will not be tracked.
    pub fn number_outs_with_amount(&self, amount: u64) -> usize {
        u64_to_usize(
            self.number_of_outputs
                .get(&amount)
                .copied()
                .unwrap_or_default(),
        )
    }

    /// Request an output with a given amount and amount index from the cache.
    pub fn get_output(&self, amount: u64, index: u64) -> Option<&OutputOnChain> {
        self.cached_outputs
            .get(&amount)
            .and_then(|map| map.get(&index))
    }

    /// Adds a [`Transaction`] to the cache.
    fn add_tx<const MINER_TX: bool>(&mut self, height: usize, tx: &Transaction<Pruned>) {
        for (i, out) in tx.prefix().outputs.iter().enumerate() {
            let amount = if MINER_TX && tx.version() == 2 {
                0
            } else {
                out.amount.unwrap_or_default()
            };

            let Some(outputs_with_amount) = self.number_of_outputs.get_mut(&amount) else {
                continue;
            };

            let amount_index_of_out = *outputs_with_amount;
            *outputs_with_amount += 1;

            if let Some(set) = self.wanted_outputs.get_mut(&amount) {
                if set.swap_remove(&amount_index_of_out) {
                    self.cached_outputs.entry(amount).or_default().insert(
                        amount_index_of_out,
                        OutputOnChain {
                            height,
                            time_lock: tx.prefix().additional_timelock,
                            key: out.key,
                            commitment: get_output_commitment(tx, i),
                            txid: None,
                        },
                    );
                }
            }
        }
    }

    /// Adds a block to the cache.
    ///
    /// This function will add any outputs to the cache that were requested when building the cache
    /// but were not in the DB, if they are in the block.
    pub fn add_block_to_cache(&mut self, block: &VerifiedBlockInformation) {
        self.add_tx::<true>(
            block.height,
            &block
                .block
                .miner_transaction()
                .clone()
                .pruned_with_prunable()
                .0,
        );

        for tx in &block.txs {
            self.add_tx::<false>(block.height, &tx.tx);
        }
    }
}

/// Returns the amount commitment for the output at the given index `i` in the [`Transaction`]
fn get_output_commitment(tx: &Transaction<Pruned>, i: usize) -> CompressedPoint {
    match tx {
        Transaction::V1 { prefix, .. } => {
            compute_zero_commitment(prefix.outputs[i].amount.unwrap_or_default())
        }
        Transaction::V2 { prefix, proofs } => {
            let Some(proofs) = proofs else {
                return compute_zero_commitment(prefix.outputs[i].amount.unwrap_or_default());
            };

            proofs.base.commitments[i]
        }
    }
}
