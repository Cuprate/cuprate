use crate::{OutputOnChain, VerifiedBlockInformation};
use cuprate_helper::crypto::compute_zero_commitment;
use curve25519_dalek::EdwardsPoint;
use indexmap::{IndexMap, IndexSet};
use monero_serai::transaction::Transaction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputCache {
    cached_outputs: IndexMap<u64, IndexMap<u64, OutputOnChain>>,

    number_of_outputs: IndexMap<u64, u64>,
    wanted_outputs: IndexMap<u64, IndexSet<u64>>,
}

impl OutputCache {
    pub fn new(
        cached_outputs: IndexMap<u64, IndexMap<u64, OutputOnChain>>,
        number_of_outputs: IndexMap<u64, u64>,
        wanted_outputs: IndexMap<u64, IndexSet<u64>>,
    ) -> OutputCache {
        OutputCache {
            cached_outputs,
            number_of_outputs,
            wanted_outputs,
        }
    }

    pub fn number_outs_with_amount(&self, amount: u64) -> usize {
        self.number_of_outputs
            .get(&amount)
            .copied()
            .unwrap_or_default() as usize
    }

    pub fn get_output(&self, amount: u64, index: u64) -> Option<&OutputOnChain> {
        self.cached_outputs
            .get(&amount)
            .and_then(|map| map.get(&index))
    }

    fn add_miner_tx(&mut self, height: usize, tx: &Transaction) {
        let version = tx.version();

        for outs in &tx.prefix().outputs {
            let amount = match version {
                1 => outs.amount.unwrap_or_default(),
                2 => 0,
                _ => panic!("Unknown transaction version {}", version),
            };

            let outputs_with_amount = self.number_of_outputs.entry(amount).or_default();
            let amount_index_of_out = *outputs_with_amount;
            *outputs_with_amount += 1;

            if let Some(set) = self.wanted_outputs.get_mut(&amount) {
                if set.swap_remove(&amount_index_of_out) {
                    self.cached_outputs.entry(amount).or_default().insert(
                        amount_index_of_out,
                        OutputOnChain {
                            height,
                            time_lock: tx.prefix().additional_timelock,
                            key: outs.key.decompress(),
                            commitment: compute_zero_commitment(outs.amount.unwrap_or_default()),
                        },
                    );
                }
            }
        }
    }

    fn add_tx(&mut self, height: usize, tx: &Transaction) {
        for (i, out) in tx.prefix().outputs.iter().enumerate() {
            let amount = out.amount.unwrap_or_default();

            let outputs_with_amount = self.number_of_outputs.entry(amount).or_default();
            let amount_index_of_out = *outputs_with_amount;
            *outputs_with_amount += 1;

            if let Some(set) = self.wanted_outputs.get_mut(&amount) {
                if set.swap_remove(&amount_index_of_out) {
                    self.cached_outputs.entry(amount).or_default().insert(
                        amount_index_of_out,
                        OutputOnChain {
                            height,
                            time_lock: tx.prefix().additional_timelock,
                            key: out.key.decompress(),
                            commitment: get_output_commitment(&tx, i),
                        },
                    );
                }
            }
        }
    }

    pub fn add_block_to_cache(&mut self, block: &VerifiedBlockInformation) {
        self.add_miner_tx(block.height, &block.block.miner_transaction);

        for tx in &block.txs {
            self.add_tx(block.height, &tx.tx);
        }
    }
}

fn get_output_commitment(tx: &Transaction, i: usize) -> EdwardsPoint {
    match tx {
        Transaction::V1 { prefix, .. } => {
            compute_zero_commitment(prefix.outputs[i].amount.unwrap_or_default())
        }
        Transaction::V2 { proofs, .. } => {
            proofs
                .as_ref()
                .expect("A V2 transaction with no RCT proofs is a miner tx")
                .base
                .commitments[i]
        }
    }
}
