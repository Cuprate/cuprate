use crate::config::Config;
use crate::types::{
    AltBlockHeight, AltChainInfo, AltTransactionInfo, Amount, AmountIndices, BlockHash,
    BlockHeight, CompactAltBlockInfo, Hash32Bytes, HeedAmountIndices, HeedUsize, KeyImage, Output,
    PreRctOutputId, RawChainId, StorableHeed, TxHash, TxId, ZeroKey,
};
use heed::types::U64;
use heed::{DefaultComparator, IntegerComparator};
use std::iter::{once, Once};
use std::sync::OnceLock;
use tapes::{Advice, MmapFile, Tape, Tapes};

/// The name of the ringCT outputs tape.
pub const RCT_OUTPUTS: &str = "rct_outputs";
/// The pruned blobs tape name.
pub const PRUNED_BLOBS: &str = "pruned_blobs";
/// The names of the prunable tapes, in the order of stripe.
pub const PRUNABLE_BLOBS: [&str; 8] = [
    "prunable1",
    "prunable2",
    "prunable3",
    "prunable4",
    "prunable5",
    "prunable6",
    "prunable7",
    "prunable8",
];
/// The name of the v1 prunable blobs table.
pub const V1_PRUNABLE_BLOBS: &str = "v1_prunable_blobs";
/// The name of the tx infos tape.
pub const TX_INFOS: &str = "tx_infos";
/// The name of the block infos tape.
pub const BLOCK_INFOS: &str = "block_infos";



pub struct Blockchain {
    pub(crate) dynamic_tables: heed::Env,
    pub(crate) linear_tapes: Tapes<MmapFile>,
    pub(crate) block_heights: heed::Database<Hash32Bytes, HeedUsize>,
    pub(crate) key_images: heed::Database<ZeroKey, Hash32Bytes, IntegerComparator>,
    pub(crate) pre_rct_outputs: heed::Database<
        U64<heed::byteorder::NativeEndian>,
        Output,
        IntegerComparator,
        IntegerComparator,
    >,
    pub(crate) tx_ids: heed::Database<Hash32Bytes, HeedUsize>,
    pub(crate) tx_outputs: heed::Database<HeedUsize, HeedAmountIndices, IntegerComparator>,
    pub(crate) alt_chain_infos: heed::Database<StorableHeed<RawChainId>, StorableHeed<AltChainInfo>>,
    pub(crate) alt_block_heights: heed::Database<Hash32Bytes, StorableHeed<AltBlockHeight>>,
    pub(crate) alt_blocks_info: heed::Database<StorableHeed<AltBlockHeight>, StorableHeed<CompactAltBlockInfo>>,
    pub(crate) alt_block_blobs: heed::Database<StorableHeed<AltBlockHeight>, heed::types::Bytes>,
    pub(crate) alt_transaction_blobs: heed::Database<Hash32Bytes, heed::types::Bytes>,
    pub(crate) alt_transaction_infos: heed::Database<Hash32Bytes, StorableHeed<AltTransactionInfo>>,
}

impl Drop for Blockchain {
    fn drop(&mut self) {
        tracing::info!("Syncing blockchain database to storage.");
        self.dynamic_tables.force_sync();
    }
}
