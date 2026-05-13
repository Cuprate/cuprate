use std::{borrow::Cow, collections::HashMap, sync::Mutex};

use fjall::{KeyspaceCreateOptions, PersistMode};
use monero_oxide::transaction::Transaction;
use tapes::{Persistence, TapeOpenOptions, Tapes, TapesRead};

use cuprate_helper::cast::{u64_to_usize, usize_to_u64};

use crate::{
    config::Config,
    types::{Amount, BlockInfo, RctOutput, TxInfo},
    BlockchainError,
};

/// The blockchain database.
pub struct BlockchainDatabase {
    /// The tapes database.
    pub(crate) linear_tapes: Tapes,
    /// The fjall database.
    pub(crate) fjall: fjall::Database,

    /// Block heights:
    ///
    /// | key                  | value                               |
    /// |----------------------|-------------------------------------|
    /// | block hash: [u8; 32] | block height: usize (little endian) |
    pub(crate) block_heights: fjall::Keyspace,
    /// Key images:
    ///
    /// | key                 | value |
    /// |---------------------|-------|
    /// | key image: [u8; 32] | []    |
    pub(crate) key_images: fjall::Keyspace,
    /// Pre-RCT outputs:
    ///
    /// | key                                     | value                             |
    /// |-----------------------------------------|-----------------------------------|
    /// | The ID of the output [`PreRctOutputId`] | The output data: [`Output`] bytes |
    pub(crate) pre_rct_outputs: fjall::Keyspace,
    /// Transaction IDs:
    ///
    /// | key               | value                      |
    /// |-------------------|----------------------------|
    /// | Tx hash: [u8; 32] | Tx ID: u64 (little endian) |
    pub(crate) tx_ids: fjall::Keyspace,
    /// V1 transaction output amount indices:
    ///
    /// | key                        | value                                           |
    /// |----------------------------|--------------------------------------------------|
    /// | Tx ID: u64 (little endian) | amount indices as a [u64] (little endian) slice |
    pub(crate) v1_tx_outputs: fjall::Keyspace,
    /// Alt chain info:
    ///
    /// | key                           | value                  |
    /// |-------------------------------|------------------------|
    /// | Chain ID: u64 (little endian) | [`AltChainInfo`] bytes |
    pub(crate) alt_chain_infos: fjall::Keyspace,
    /// Alt block heights:
    ///
    /// | key                  | value                    |
    /// |----------------------|--------------------------|
    /// | block hash: [u8; 32] | [`AltBlockHeight`] bytes |
    pub(crate) alt_block_heights: fjall::Keyspace,
    /// Alt block info:
    ///
    /// | key                        | value                          |
    /// |----------------------------|--------------------------------|
    /// | [`AltBlockHeight`] bytes   | [`CompactAltBlockInfo`] bytes  |
    pub(crate) alt_block_infos: fjall::Keyspace,
    /// Alt block blobs:
    ///
    /// | key                      | value            |
    /// |--------------------------|------------------|
    /// | [`AltBlockHeight`] bytes | block blob: [u8] |
    pub(crate) alt_block_blobs: fjall::Keyspace,
    /// Alt transaction blobs:
    ///
    /// | key                        | value                       |
    /// |----------------------------|-----------------------------|
    /// | transaction hash: [u8; 32] | full transaction blob: [u8] |
    pub(crate) alt_transaction_blobs: fjall::Keyspace,
    /// Alt transaction info:
    ///
    /// | key                        | value                        |
    /// |----------------------------|------------------------------|
    /// | transaction hash: [u8; 32] | [`AltTransactionInfo`] bytes |
    pub(crate) alt_transaction_infos: fjall::Keyspace,

    /// RCT (v2+) outputs, indexed sequentially.
    ///
    /// | index                 | value         |
    /// |-----------------------|---------------|
    /// | RCT output index: u64 | [`RctOutput`] |
    pub(crate) rct_outputs: tapes::FixedSizedTape<RctOutput>,
    /// Transaction info, indexed by [`TxId`].
    ///
    /// | index      | value      |
    /// |------------|------------|
    /// | Tx ID: u64 | [`TxInfo`] |
    pub(crate) tx_infos: tapes::FixedSizedTape<TxInfo>,
    /// Block info, indexed by block height.
    ///
    /// | index             | value         |
    /// |-------------------|---------------|
    /// | Block height: u64 | [`BlockInfo`] |
    pub(crate) block_infos: tapes::FixedSizedTape<BlockInfo>,
    /// Pruned blobs.
    ///
    /// The format for this blob-tape per each block is:
    ///
    /// | data                                       |
    /// |--------------------------------------------|
    /// | block blob (header, miner tx, tx hashes)   |
    /// | tx 0 pruned blob                           |
    /// | tx 0 prunable hash (32 bytes)              |
    /// | tx 1 pruned blob                           |
    /// | tx 1 prunable hash (32 bytes)              |
    /// | ...                                        |
    ///
    /// The prunable hash is `[0; 32]` for v1 txs.
    /// Each block is appended directly after the one before it.
    pub(crate) pruned_blobs: tapes::BlobTape,
    /// V1 prunable transaction blobs, indexed by [`TxInfo::prunable_blob_idx`].
    ///
    /// This tape stores the prunable blob for all V1 txs, these can't be pruned.
    pub(crate) v1_prunable_blobs: tapes::BlobTape,
    /// V2+ prunable transaction blobs, split across 8 stripes.
    /// Indexed by [`TxInfo::prunable_blob_idx`].
    ///
    /// These tapes store the prunable part of each tx, the stripe a tx is stored in depends on the
    /// height of the block.
    pub(crate) prunable_blobs: Vec<tapes::BlobTape>,

    /// A runtime cache of the number of outputs for each pre-rct output amount.
    /// This is filled in lazily.
    pub(crate) pre_rct_numb_outputs_cache: Mutex<HashMap<Amount, u64>>,
}

impl BlockchainDatabase {
    /// Open a [`BlockchainDatabase`] with an [`fjall::Database`] for storing data that can't be stored in tapes.
    pub fn open_with_fjall_database(
        config: &Config,
        fjall: fjall::Database,
    ) -> Result<Self, BlockchainError> {
        let block_heights = fjall.keyspace("block_heights", KeyspaceCreateOptions::default)?;
        let key_images = fjall.keyspace("key_images", KeyspaceCreateOptions::default)?;
        let pre_rct_outputs = fjall.keyspace("pre_rct_outputs", KeyspaceCreateOptions::default)?;
        let tx_ids = fjall.keyspace("tx_ids", KeyspaceCreateOptions::default)?;
        let v1_tx_outputs = fjall.keyspace("tx_outputs", KeyspaceCreateOptions::default)?;

        let alt_chain_infos = fjall.keyspace("alt_chain_infos", KeyspaceCreateOptions::default)?;
        let alt_block_heights =
            fjall.keyspace("alt_block_heights", KeyspaceCreateOptions::default)?;
        let alt_block_infos = fjall.keyspace("alt_block_infos", KeyspaceCreateOptions::default)?;
        let alt_block_blobs = fjall.keyspace("alt_block_blobs", KeyspaceCreateOptions::default)?;
        let alt_transaction_blobs =
            fjall.keyspace("alt_transaction_blobs", KeyspaceCreateOptions::default)?;
        let alt_transaction_infos =
            fjall.keyspace("alt_transaction_infos", KeyspaceCreateOptions::default)?;

        let tapes_index_dir = config.index_dir.join("tapes");
        let tapes_blob_dir = config.blob_dir.join("tapes");

        let linear_tapes = Tapes::open(&tapes_index_dir)?;
        let mut tape_append_tx = linear_tapes.append();

        let rct_outputs = tape_append_tx.open_fixed_sized_tape(
            "rct_outputs",
            &TapeOpenOptions {
                top_cache_size: config.cache_sizes.rct_outputs,
                dir: tapes_index_dir.clone(),
            },
        )?;
        let tx_infos = tape_append_tx.open_fixed_sized_tape(
            "tx_infos",
            &TapeOpenOptions {
                top_cache_size: config.cache_sizes.tx_infos,
                dir: tapes_index_dir.clone(),
            },
        )?;
        let block_infos = tape_append_tx.open_fixed_sized_tape(
            "block_infos",
            &TapeOpenOptions {
                top_cache_size: config.cache_sizes.block_infos,
                dir: tapes_index_dir,
            },
        )?;
        let pruned_blobs = tape_append_tx.open_blob_tape(
            "pruned_blobs",
            &TapeOpenOptions {
                top_cache_size: config.cache_sizes.pruned_blobs,
                dir: tapes_blob_dir.clone(),
            },
        )?;
        let v1_prunable_blobs = tape_append_tx.open_blob_tape(
            "v1_prunable_blobs",
            &TapeOpenOptions {
                top_cache_size: config.cache_sizes.v1_prunable_blobs,
                dir: tapes_blob_dir.clone(),
            },
        )?;

        const PRUNABLE_BLOBS: [&str; 8] = [
            "prunable1",
            "prunable2",
            "prunable3",
            "prunable4",
            "prunable5",
            "prunable6",
            "prunable7",
            "prunable8",
        ];

        let prunable_blobs = (0..8)
            .map(|i| {
                tape_append_tx.open_blob_tape(
                    PRUNABLE_BLOBS[i],
                    &TapeOpenOptions {
                        top_cache_size: config.cache_sizes.prunable_blobs,
                        dir: tapes_blob_dir.clone(),
                    },
                )
            })
            .collect::<Result<_, _>>()?;

        tape_append_tx.commit(Persistence::SyncAll)?;

        drop(tape_append_tx);

        tracing::debug!("opened db");
        Ok(Self {
            fjall,
            linear_tapes,
            block_heights,
            key_images,
            pre_rct_outputs,
            tx_ids,
            v1_tx_outputs,
            alt_chain_infos,
            alt_block_heights,
            alt_block_infos,
            alt_block_blobs,
            alt_transaction_blobs,
            alt_transaction_infos,
            rct_outputs,
            tx_infos,
            block_infos,
            pruned_blobs,
            v1_prunable_blobs,
            prunable_blobs,
            pre_rct_numb_outputs_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Checks if the fjall and tapes database are in sync and rebuilds the fjall database if it
    /// is not.
    pub fn make_consistent(&self) -> Result<(), BlockchainError> {
        tracing::info!("Checking blockchain database consistency.");

        let tapes_reader = self.linear_tapes.reader();

        if tapes_reader
            .fixed_sized_tape_len(&self.block_infos)
            .expect("block_infos tape exists")
            != usize_to_u64(self.block_heights.len()?)
        {
            tracing::warn!("fjall and tapes are out of sync");
            self.rebuild_fjall_database()?;
        }

        Ok(())
    }

    /// Rebuilds the fjall database.
    pub fn rebuild_fjall_database(&self) -> Result<(), BlockchainError> {
        self.block_heights.clear()?;
        self.key_images.clear()?;
        self.pre_rct_outputs.clear()?;
        self.tx_ids.clear()?;
        self.v1_tx_outputs.clear()?;
        self.alt_chain_infos.clear()?;
        self.alt_block_heights.clear()?;
        self.alt_block_infos.clear()?;
        self.alt_block_blobs.clear()?;
        self.alt_transaction_blobs.clear()?;
        self.alt_transaction_infos.clear()?;

        let rebuild_span = tracing::info_span!("rebuild_fjall_database");
        let _guard = rebuild_span.enter();

        tracing::info!("rebuilding fjall db");

        let tapes_reader = self.linear_tapes.reader();

        let tx_infos_iter = tapes_reader.iter_from(&self.tx_infos, 0)?;
        let mut tx_iter = tx_infos_iter.map(|tx_info| {
            let tx_info = tx_info.unwrap();

            let mut tx_blob = vec![0; tx_info.pruned_size];
            tapes_reader
                .read_bytes(&self.pruned_blobs, tx_info.pruned_blob_idx, &mut tx_blob)
                .unwrap();

            let tx = Transaction::read(&mut tx_blob.as_slice()).unwrap();

            Cow::Owned(tx)
        });

        let mut batch = self.fjall.batch().durability(Some(PersistMode::Buffer));
        let mut numb_txs = 0;
        for height in 0..tapes_reader
            .fixed_sized_tape_len(&self.block_infos)
            .expect("block_infos tape exists")
        {
            let block =
                crate::ops::block::get_block(&u64_to_usize(height), None, &tapes_reader, self)?;

            let _miner_tx = tx_iter.next();

            crate::ops::block::add_block_to_dynamic_tables(
                self,
                &block,
                &block.hash(),
                &mut tx_iter,
                &mut numb_txs,
                &mut batch,
                &mut self.pre_rct_numb_outputs_cache.lock().unwrap(),
            )?;

            if height % 1000 == 0 {
                tracing::info!("{} blocks processed", height);
                let old_batch = std::mem::replace(
                    &mut batch,
                    self.fjall.batch().durability(Some(PersistMode::Buffer)),
                );

                old_batch.commit()?;
            }
        }

        batch.commit()?;

        Ok(())
    }
}

impl Drop for BlockchainDatabase {
    fn drop(&mut self) {
        tracing::info!(parent: &tracing::Span::none(), "Syncing blockchain database to storage.");

        let _ = self.fjall.persist(PersistMode::SyncAll);

        let _ = self.linear_tapes.append().commit(Persistence::SyncAll);
    }
}
