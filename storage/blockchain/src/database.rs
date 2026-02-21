use std::{
    borrow::Cow,
    collections::HashMap,
    iter::{once, Once},
    sync::{Mutex, OnceLock},
};

use fjall::{KeyspaceCreateOptions, PersistMode};
use itertools::Itertools;
use monero_oxide::transaction::Transaction;
use tapes::{Persistence, TapeOpenOptions, Tapes, TapesRead};

use crate::{
    config::Config,
    types::{
        AltBlockHeight, AltChainInfo, AltTransactionInfo, Amount, AmountIndices, BlockHash,
        BlockHeight, BlockInfo, CompactAltBlockInfo, KeyImage, Output, PreRctOutputId, RawChainId,
        RctOutput, TxHash, TxId, TxInfo,
    },
    BlockchainError,
};

pub struct BlockchainDatabase {
    pub(crate) linear_tapes: Tapes,
    pub(crate) fjall_keyspace: fjall::Database,

    pub(crate) block_heights: fjall::Keyspace,
    pub(crate) key_images: fjall::Keyspace,
    pub(crate) pre_rct_outputs: fjall::Keyspace,
    pub(crate) tx_ids: fjall::Keyspace,
    pub(crate) v1_tx_outputs: fjall::Keyspace,
    pub(crate) alt_chain_infos: fjall::Keyspace,
    pub(crate) alt_block_heights: fjall::Keyspace,
    pub(crate) alt_blocks_info: fjall::Keyspace,
    pub(crate) alt_block_blobs: fjall::Keyspace,
    pub(crate) alt_transaction_blobs: fjall::Keyspace,
    pub(crate) alt_transaction_infos: fjall::Keyspace,

    pub(crate) rct_outputs: tapes::FixedSizedTape<RctOutput>,
    pub(crate) tx_infos: tapes::FixedSizedTape<TxInfo>,
    pub(crate) block_infos: tapes::FixedSizedTape<BlockInfo>,
    pub(crate) pruned_blobs: tapes::BlobTape,
    pub(crate) v1_prunable_blobs: tapes::BlobTape,
    pub(crate) prunable_blobs: Vec<tapes::BlobTape>,

    pub(crate) pre_rct_numb_outputs_cache: Mutex<HashMap<Amount, u64>>,
}

impl BlockchainDatabase {
    pub fn open_with_fjall_database(
        config: Config,
        fjall_keyspace: fjall::Database,
    ) -> Result<Self, BlockchainError> {
        let block_heights = fjall_keyspace
            .keyspace("block_heights", || KeyspaceCreateOptions::default())
            .unwrap();
        let key_images = fjall_keyspace
            .keyspace("key_images", || KeyspaceCreateOptions::default())
            .unwrap();
        let pre_rct_outputs = fjall_keyspace
            .keyspace("pre_rct_output", || KeyspaceCreateOptions::default())
            .unwrap();
        let tx_ids = fjall_keyspace
            .keyspace("tx_ids", || KeyspaceCreateOptions::default())
            .unwrap();
        let v1_tx_outputs = fjall_keyspace
            .keyspace("tx_outputs", || KeyspaceCreateOptions::default())
            .unwrap();

        let alt_chain_infos = fjall_keyspace
            .keyspace("alt_chain_infos", || KeyspaceCreateOptions::default())
            .unwrap();
        let alt_block_heights = fjall_keyspace
            .keyspace("alt_block_heights", || KeyspaceCreateOptions::default())
            .unwrap();
        let alt_blocks_info = fjall_keyspace
            .keyspace("alt_blocks_infos", || KeyspaceCreateOptions::default())
            .unwrap();
        let alt_block_blobs = fjall_keyspace
            .keyspace("alt_block_blobs", || KeyspaceCreateOptions::default())
            .unwrap();
        let alt_transaction_blobs = fjall_keyspace
            .keyspace("alt_transaction_blobs", || KeyspaceCreateOptions::default())
            .unwrap();
        let alt_transaction_infos = fjall_keyspace
            .keyspace("alt_transaction_infos", || KeyspaceCreateOptions::default())
            .unwrap();

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
                dir: tapes_index_dir.clone(),
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
            fjall_keyspace,
            linear_tapes,
            block_heights,
            key_images,
            pre_rct_outputs,
            tx_ids,
            v1_tx_outputs,
            alt_chain_infos,
            alt_block_heights,
            alt_blocks_info,
            alt_block_blobs,
            alt_transaction_blobs,
            alt_transaction_infos,
            rct_outputs,
            tx_infos,
            block_infos,
            pruned_blobs,
            v1_prunable_blobs,
            prunable_blobs,
            pre_rct_numb_outputs_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        })
    }

    pub fn make_consistent(&self) -> Result<(), BlockchainError> {
        tracing::info!("Checking blockchain database consistency.");

        let tapes_reader = self.linear_tapes.reader();

        if tapes_reader
            .fixed_sized_tape_len(&self.block_infos)
            .expect("block_infos tape exists")
            != self.block_heights.len()? as u64
        {
            tracing::warn!("fjall and tapes are out of sync");
            self.rebuild_fjall_database()?;
        }

        Ok(())
    }

    pub fn rebuild_fjall_database(&self) -> Result<(), BlockchainError> {
        self.block_heights.clear()?;
        self.key_images.clear()?;
        self.pre_rct_outputs.clear()?;
        self.tx_ids.clear()?;
        self.v1_tx_outputs.clear()?;
        self.alt_chain_infos.clear()?;
        self.alt_block_heights.clear()?;
        self.alt_blocks_info.clear()?;
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

            let mut tx_blob = vec![0; tx_info.pruned_size as usize];
            tapes_reader
                .read_bytes(&self.pruned_blobs, tx_info.pruned_blob_idx, &mut tx_blob)
                .unwrap();

            let tx = Transaction::read(&mut tx_blob.as_slice()).unwrap();

            Cow::Owned(tx)
        });

        let mut batch = self
            .fjall_keyspace
            .batch()
            .durability(Some(PersistMode::Buffer));
        let mut numb_txs = 0;
        for height in 0..tapes_reader
            .fixed_sized_tape_len(&self.block_infos)
            .expect("block_infos tape exists")
        {
            let block =
                crate::ops::block::get_block(&(height as usize), None, &tapes_reader, self)?;

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
                    self.fjall_keyspace
                        .batch()
                        .durability(Some(PersistMode::Buffer)),
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
        tracing::info!("Syncing blockchain database to storage.");

        self.fjall_keyspace.persist(PersistMode::SyncAll);

        self.linear_tapes.append().commit(Persistence::SyncAll);
    }
}
