use std::collections::HashMap;
use crate::config::Config;
use crate::types::{AltBlockHeight, AltChainInfo, AltTransactionInfo, Amount, AmountIndices, BlockHash, BlockHeight, BlockInfo, CompactAltBlockInfo, KeyImage, Output, PreRctOutputId, RawChainId, RctOutput, TxHash, TxId, TxInfo};
use std::iter::{once, Once};
use std::sync::{Mutex, OnceLock};
use fjall::PersistMode;
use tapes::Tapes;

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
    pub(crate) linear_tapes: Tapes,
    pub(crate) fjall_keyspace: fjall::SingleWriterTxDatabase,

    pub(crate) block_heights_fjall: fjall::SingleWriterTxKeyspace,
    pub(crate) key_images_fjall: fjall::SingleWriterTxKeyspace,
    pub(crate) pre_rct_outputs_fjall: fjall::SingleWriterTxKeyspace,
    pub(crate) tx_ids_fjall: fjall::SingleWriterTxKeyspace,
    pub(crate) tx_outputs_fjall: fjall::SingleWriterTxKeyspace,

    pub(crate) rct_outputs: tapes::FixedSizedTape<RctOutput>,
    pub(crate) tx_infos: tapes::FixedSizedTape<TxInfo>,
    pub(crate) block_infos: tapes::FixedSizedTape<BlockInfo>,
    pub(crate) pruned_blobs: tapes::BlobTape,
    pub(crate) v1_prunable_blobs: tapes::BlobTape,
    pub(crate) prunable_blobs: Vec<tapes::BlobTape>,
    

    pub(crate) pre_rct_numb_outputs_cache: Mutex<HashMap<Amount, u64>>,

/*
    pub(crate) alt_chain_infos: heed::Database<StorableHeed<RawChainId>, StorableHeed<AltChainInfo>>,
    pub(crate) alt_block_heights: heed::Database<Hash32Bytes, StorableHeed<AltBlockHeight>>,
    pub(crate) alt_blocks_info: heed::Database<StorableHeed<AltBlockHeight>, StorableHeed<CompactAltBlockInfo>>,
    pub(crate) alt_block_blobs: heed::Database<StorableHeed<AltBlockHeight>, heed::types::Bytes>,
    pub(crate) alt_transaction_blobs: heed::Database<Hash32Bytes, heed::types::Bytes>,
    pub(crate) alt_transaction_infos: heed::Database<Hash32Bytes, StorableHeed<AltTransactionInfo>>,
    
 */
}

impl Drop for Blockchain {
    fn drop(&mut self) {
        tracing::info!("Syncing blockchain database to storage.");
        self.fjall_keyspace.persist(PersistMode::SyncAll);
    }
}
