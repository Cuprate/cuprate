use crate::config::Config;
use crate::types::{
    AltBlockHeight, AltChainInfo, AltTransactionInfo, Amount, AmountIndices, BlockHash,
    BlockHeight, CompactAltBlockInfo, Hash32Bytes, HeedAmountIndices, HeedUsize, KeyImage, Output,
    PreRctOutputId, RawChainId, StorableHeed, TxHash, TxId, ZeroKey,
};
use tapes::{Advice, Tapes, Tape, MmapFile};
use heed::types::U64;
use heed::{DefaultComparator, IntegerComparator};
use std::iter::{once, Once};
use std::sync::OnceLock;

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

pub static BLOCK_HEIGHTS: OnceLock<heed::Database<Hash32Bytes, HeedUsize>> = OnceLock::new();

pub static KEY_IMAGES: OnceLock<heed::Database<ZeroKey, Hash32Bytes, IntegerComparator>> =
    OnceLock::new();

pub static PRE_RCT_OUTPUTS: OnceLock<
    heed::Database<
        U64<heed::byteorder::NativeEndian>,
        Output,
        IntegerComparator,
        IntegerComparator,
    >,
> = OnceLock::new();

pub static TX_IDS: OnceLock<heed::Database<Hash32Bytes, HeedUsize>> = OnceLock::new();

pub static TX_OUTPUTS: OnceLock<heed::Database<HeedUsize, HeedAmountIndices, IntegerComparator>> =
    OnceLock::new();

pub static ALT_CHAIN_INFOS: OnceLock<
    heed::Database<StorableHeed<RawChainId>, StorableHeed<AltChainInfo>>,
> = OnceLock::new();

pub static ALT_BLOCK_HEIGHTS: OnceLock<heed::Database<Hash32Bytes, StorableHeed<AltBlockHeight>>> =
    OnceLock::new();

pub static ALT_BLOCKS_INFO: OnceLock<
    heed::Database<StorableHeed<AltBlockHeight>, StorableHeed<CompactAltBlockInfo>>,
> = OnceLock::new();

pub static ALT_BLOCK_BLOBS: OnceLock<
    heed::Database<StorableHeed<AltBlockHeight>, heed::types::Bytes>,
> = OnceLock::new();

pub static ALT_TRANSACTION_BLOBS: OnceLock<heed::Database<Hash32Bytes, heed::types::Bytes>> =
    OnceLock::new();

pub static ALT_TRANSACTION_INFOS: OnceLock<
    heed::Database<Hash32Bytes, StorableHeed<AltTransactionInfo>>,
> = OnceLock::new();

pub struct Blockchain {
    pub(crate) dynamic_tables: heed::Env,
    pub(crate) linear_tapes: Tapes<MmapFile>,
}

impl Drop for Blockchain {
    fn drop(&mut self) {
        tracing::info!("Syncing blockchain database to storage.");
        self.dynamic_tables.force_sync();
    }
}
