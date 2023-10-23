use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    sync::Arc,
};

use monero_serai::{
    block::Block,
    transaction::{Timelock, Transaction},
};

use crate::transactions::TransactionVerificationData;
use cuprate_common::Network;

/// A cache which can keep chain state while scanning.
///
/// Because we are using a RPC interface with a node we need to keep track
/// of certain data that the node doesn't hold or give us like the number
/// of outputs at a certain time.
#[derive(Debug, Default, Clone)]
pub struct ScanningCache {
    network: Network,
    numb_outs: HashMap<u64, u64>,
    time_locked_out: HashMap<[u8; 32], Timelock>,
    pub already_generated_coins: u64,
    /// The height of the *next* block to scan.
    pub height: u64,
}

impl ScanningCache {
    pub fn add_new_block_data(
        &mut self,
        generated_coins: u64,
        miner_tx: &Transaction,
        txs: &[Arc<TransactionVerificationData>],
    ) {
        self.add_tx_time_lock(miner_tx.hash(), miner_tx.prefix.timelock);
        miner_tx
            .prefix
            .outputs
            .iter()
            .for_each(|out| self.add_outs(out.amount.unwrap_or(0), 1));

        txs.iter().for_each(|tx| {
            self.add_tx_time_lock(tx.tx_hash, tx.tx.prefix.timelock);
            tx.tx
                .prefix
                .outputs
                .iter()
                .for_each(|out| self.add_outs(out.amount.unwrap_or(0), 1));
        });

        self.already_generated_coins = self.already_generated_coins.saturating_add(generated_coins);
        self.height += 1;
    }

    pub fn outputs_time_lock(&self, tx: &[u8; 32]) -> Timelock {
        self.time_locked_out
            .get(tx)
            .copied()
            .unwrap_or(Timelock::None)
    }

    pub fn add_tx_time_lock(&mut self, tx: [u8; 32], time_lock: Timelock) {
        match time_lock {
            Timelock::None => (),
            lock => {
                self.time_locked_out.insert(tx, lock);
            }
        }
    }

    pub fn total_outs(&self) -> u64 {
        self.numb_outs.values().sum()
    }

    pub fn numb_outs(&self, amount: u64) -> u64 {
        *self.numb_outs.get(&amount).unwrap_or(&0)
    }

    pub fn add_outs(&mut self, amount: u64, count: u64) {
        if let Some(numb_outs) = self.numb_outs.get_mut(&amount) {
            *numb_outs += count;
        } else {
            self.numb_outs.insert(amount, count);
        }
    }
}

impl Display for ScanningCache {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let rct_outs = self.numb_outs(0);
        let total_outs = self.total_outs();

        f.debug_struct("Cache")
            .field("next_block", &self.height)
            .field("rct_outs", &rct_outs)
            .field("total_outs", &total_outs)
            .finish()
    }
}
