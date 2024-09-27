use cuprate_types::VerifiedBlockInformation;

use cuprate_helper::{cast::usize_to_u64, map::split_u128_into_low_high_bits};

use crate::misc::BlockHeader;

impl From<&VerifiedBlockInformation> for BlockHeader {
    fn from(b: &VerifiedBlockInformation) -> Self {
        let (cumulative_difficulty_top64, cumulative_difficulty) =
            split_u128_into_low_high_bits(b.cumulative_difficulty);

        Self {
            block_size: usize_to_u64(b.block_blob.len()),
            block_weight: usize_to_u64(b.weight),
            cumulative_difficulty_top64,
            cumulative_difficulty,
            depth: todo!(),
            difficulty_top64: todo!(),
            difficulty: todo!(),
            hash: hex::encode(b.block_hash),
            height: usize_to_u64(b.height),
            long_term_weight: usize_to_u64(b.long_term_weight),
            major_version: b.block.header.hardfork_version,
            miner_tx_hash: hex::encode(b.block.miner_transaction.hash()),
            minor_version: b.block.header.hardfork_signal,
            nonce: b.block.header.nonce,
            num_txes: usize_to_u64(b.txs.len()),
            orphan_status: todo!(),
            pow_hash: hex::encode(b.pow_hash),
            prev_hash: hex::encode(b.block.header.previous),
            reward: todo!(),
            timestamp: b.block.header.timestamp,
            wide_cumulative_difficulty: todo!(),
            wide_difficulty: todo!(),
        }
    }
}
