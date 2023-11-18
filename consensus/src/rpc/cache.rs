use std::io::Write;
use std::{
    collections::HashMap,
    collections::HashSet,
    fmt::{Display, Formatter},
    io::BufWriter,
    path::Path,
    sync::Arc,
};

use bincode::{Decode, Encode};
use monero_serai::transaction::{Input, Timelock, Transaction};
use tracing_subscriber::fmt::MakeWriter;

use crate::transactions::TransactionVerificationData;

/// A cache which can keep chain state while scanning.
///
/// Because we are using a RPC interface with a node we need to keep track
/// of certain data that the node doesn't hold or give us like the number
/// of outputs at a certain time.
#[derive(Debug, Default, Clone, Encode, Decode)]
pub struct ScanningCache {
    //    network: u8,
    numb_outs: HashMap<u64, usize>,
    time_locked_out: HashMap<[u8; 32], u64>,
    kis: HashSet<[u8; 32]>,
    pub already_generated_coins: u64,
    /// The height of the *next* block to scan.
    pub height: u64,
}

impl ScanningCache {
    pub fn save(&self, file: &Path) -> Result<(), tower::BoxError> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(file)?;
        let mut writer = BufWriter::new(file.make_writer());
        bincode::encode_into_std_write(self, &mut writer, bincode::config::standard())?;
        writer.flush()?;
        Ok(())
    }

    pub fn load(file: &Path) -> Result<ScanningCache, tower::BoxError> {
        let mut file = std::fs::OpenOptions::new().read(true).open(file)?;

        bincode::decode_from_std_read(&mut file, bincode::config::standard()).map_err(Into::into)
    }

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

            tx.tx.prefix.inputs.iter().for_each(|inp| match inp {
                Input::ToKey { key_image, .. } => {
                    assert!(self.kis.insert(key_image.compress().to_bytes()))
                }
                _ => unreachable!(),
            })
        });

        self.already_generated_coins = self.already_generated_coins.saturating_add(generated_coins);
        self.height += 1;
    }

    /// Returns true if any kis are included in our spent set.
    pub fn are_kis_spent(&self, kis: HashSet<[u8; 32]>) -> bool {
        !self.kis.is_disjoint(&kis)
    }

    pub fn outputs_time_lock(&self, tx: &[u8; 32]) -> Timelock {
        let time_lock = self.time_locked_out.get(tx).copied().unwrap_or(0);
        match time_lock {
            0 => Timelock::None,
            block if block < 500_000_000 => Timelock::Block(block as usize),
            time => Timelock::Time(time),
        }
    }

    pub fn add_tx_time_lock(&mut self, tx: [u8; 32], time_lock: Timelock) {
        match time_lock {
            Timelock::None => (),
            lock => {
                self.time_locked_out.insert(
                    tx,
                    match lock {
                        Timelock::None => unreachable!(),
                        Timelock::Block(x) => x as u64,
                        Timelock::Time(x) => x,
                    },
                );
            }
        }
    }

    pub fn total_outs(&self) -> usize {
        self.numb_outs.values().sum()
    }

    pub fn numb_outs(&self, amount: u64) -> usize {
        *self.numb_outs.get(&amount).unwrap_or(&0)
    }

    pub fn add_outs(&mut self, amount: u64, count: usize) {
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
