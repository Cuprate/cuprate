use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use arc_swap::ArcSwap;
use cuprate_constants::block::MAX_BLOCK_HEIGHT_USIZE;
use cuprate_pruning::{PruningSeed, CRYPTONOTE_PRUNING_LOG_STRIPES, CRYPTONOTE_PRUNING_TIP_BLOCKS};
use fjall::{KeyspaceCreateOptions, PersistMode, Readable};
use monero_oxide::transaction::Transaction;
use rand::Rng;
use tapes::{Persistence, TapeOpenOptions, Tapes, TapesRead, TapesReadTransaction};

use cuprate_helper::cast::{u32_to_usize, u64_to_usize};

use crate::{
    config::Config,
    metadata::Metadata,
    ops::blockchain::chain_height,
    types::{Amount, BlockHeight, BlockInfo, RctOutput, TxId, TxInfo},
    BlockchainError,
};

/// The key used to store the main-chain tip in [`BlockchainDatabase::chain_tip`].
pub(crate) const CHAIN_TIP_KEY: &[u8] = b"tip";

/// Deletes a [`fjall::Keyspace`] and recreates it with the same name.
fn recreate_fjall_keyspace(
    database: &fjall::Database,
    keyspace: &fjall::Keyspace,
) -> Result<fjall::Keyspace, BlockchainError> {
    let name = keyspace.name().to_string();

    database.delete_keyspace(keyspace.clone())?;
    Ok(database.keyspace(&name, KeyspaceCreateOptions::default)?)
}

/// Deletes a [`fjall::Keyspace`] and recreates it with the same name.
pub(crate) fn reset_fjall_keyspace(
    database: &fjall::Database,
    keyspace: &ArcSwap<fjall::Keyspace>,
) -> Result<(), BlockchainError> {
    let new_keyspace = recreate_fjall_keyspace(database, &keyspace.load())?;
    keyspace.store(Arc::new(new_keyspace));

    Ok(())
}

/// The blockchain database.
pub struct BlockchainDatabase {
    /// The database configuration.
    pub(crate) config: Config,

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
    /// Main-chain tip:
    ///
    /// | key               | value                |
    /// |-------------------|----------------------|
    /// | [`CHAIN_TIP_KEY`] | block hash: [u8; 32] |
    pub(crate) chain_tip: fjall::Keyspace,
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
    pub(crate) alt_chain_infos: ArcSwap<fjall::Keyspace>,
    /// Alt block heights:
    ///
    /// | key                  | value                    |
    /// |----------------------|--------------------------|
    /// | block hash: [u8; 32] | [`AltBlockHeight`] bytes |
    pub(crate) alt_block_heights: ArcSwap<fjall::Keyspace>,
    /// Alt block info:
    ///
    /// | key                        | value                          |
    /// |----------------------------|--------------------------------|
    /// | [`AltBlockHeight`] bytes   | [`CompactAltBlockInfo`] bytes  |
    pub(crate) alt_block_infos: ArcSwap<fjall::Keyspace>,
    /// Alt block blobs:
    ///
    /// | key                      | value            |
    /// |--------------------------|------------------|
    /// | [`AltBlockHeight`] bytes | block blob: [u8] |
    pub(crate) alt_block_blobs: ArcSwap<fjall::Keyspace>,
    /// Alt transaction blobs:
    ///
    /// | key                        | value                       |
    /// |----------------------------|-----------------------------|
    /// | transaction hash: [u8; 32] | full transaction blob: [u8] |
    pub(crate) alt_transaction_blobs: ArcSwap<fjall::Keyspace>,
    /// Alt transaction info:
    ///
    /// | key                        | value                        |
    /// |----------------------------|------------------------------|
    /// | transaction hash: [u8; 32] | [`AltTransactionInfo`] bytes |
    pub(crate) alt_transaction_infos: ArcSwap<fjall::Keyspace>,

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
    ///
    /// Each blob tape is stored in an [`Option`] to allow for pruning.
    pub(crate) prunable_blobs: Vec<Option<tapes::BlobTape>>,

    /// Includes the top 5500 blocks, since pruned nodes have to always keep this.
    ///
    /// | key                        | value                  |
    /// |----------------------------|------------------------|
    /// | Tx ID: u64 (little endian) | prunable blob: [u8]    |
    pub(crate) prunable_tip: fjall::Keyspace,

    /// A runtime cache of the number of outputs for each pre-rct output amount.
    /// This is filled in lazily.
    pub(crate) pre_rct_numb_outputs_cache: Mutex<HashMap<Amount, u64>>,

    /// The pruning seed of the blockchain.
    pub(crate) metadata: Metadata,
}

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

impl BlockchainDatabase {
    /// Open a [`BlockchainDatabase`] with an [`fjall::Database`] for storing data that can't be stored in tapes.
    pub fn open_with_fjall_database(
        config: &Config,
        fjall: fjall::Database,
    ) -> Result<Self, BlockchainError> {
        let metadata = Metadata::get_or_create(&config.index_dir)?;

        let block_heights = fjall.keyspace("block_heights", KeyspaceCreateOptions::default)?;
        let chain_tip = fjall.keyspace("chain_tip", KeyspaceCreateOptions::default)?;
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
        let prunable_tip = fjall.keyspace("prunable_tip", KeyspaceCreateOptions::default)?;

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

        let prunable_tape_open_options = TapeOpenOptions {
            top_cache_size: config.cache_sizes.prunable_blobs,
            dir: tapes_blob_dir,
        };

        let prunable_blobs =
            if config.prune || metadata.get_pruning_seed() == PruningSeed::NotPruned {
                (0..PRUNABLE_BLOBS.len())
                    .map(|i| {
                        Some(
                            tape_append_tx
                                .open_blob_tape(PRUNABLE_BLOBS[i], &prunable_tape_open_options),
                        )
                        .transpose()
                    })
                    .collect::<Result<Vec<Option<_>>, _>>()
            } else {
                let stripe_idx = metadata
                    .get_pruning_seed()
                    .get_stripe()
                    .expect("we are pruning") as usize;

                (1..=PRUNABLE_BLOBS.len())
                    .map(|i| {
                        (i == stripe_idx)
                            .then(|| {
                                // don't eagerly evaluate
                                tape_append_tx.open_blob_tape(
                                    PRUNABLE_BLOBS[stripe_idx - 1],
                                    &prunable_tape_open_options,
                                )
                            })
                            .transpose()
                    })
                    .collect()
            }?;
        tape_append_tx.commit(Persistence::SyncAll)?;

        tracing::debug!("opened db");
        Ok(Self {
            fjall,
            linear_tapes,
            config: config.clone(),
            block_heights,
            chain_tip,
            key_images,
            pre_rct_outputs,
            tx_ids,
            v1_tx_outputs,
            alt_chain_infos: ArcSwap::from_pointee(alt_chain_infos),
            alt_block_heights: ArcSwap::from_pointee(alt_block_heights),
            alt_block_infos: ArcSwap::from_pointee(alt_block_infos),
            alt_block_blobs: ArcSwap::from_pointee(alt_block_blobs),
            alt_transaction_blobs: ArcSwap::from_pointee(alt_transaction_blobs),
            alt_transaction_infos: ArcSwap::from_pointee(alt_transaction_infos),
            rct_outputs,
            tx_infos,
            block_infos,
            pruned_blobs,
            v1_prunable_blobs,
            prunable_blobs,
            prunable_tip,
            pre_rct_numb_outputs_cache: Mutex::new(HashMap::new()),
            metadata,
        })
    }

    /// Returns whether Fjall and Tapes are at the same main-chain tip.
    fn tips_match(
        &self,
        fjall: &impl Readable,
        tapes: &impl TapesRead,
    ) -> Result<bool, BlockchainError> {
        let tapes_height = tapes
            .fixed_sized_tape_len(&self.block_infos)
            .expect("block_infos tape exists");
        let tapes_tip = match tapes_height.checked_sub(1) {
            Some(top_height) => Some(
                tapes
                    .read_entry(&self.block_infos, top_height)?
                    .ok_or(BlockchainError::NotFound)?
                    .block_hash,
            ),
            None => None,
        };
        let fjall_tip = fjall.get(&self.chain_tip, CHAIN_TIP_KEY)?;

        Ok(match (tapes_tip, fjall_tip.as_deref()) {
            (None, None) => true,
            (Some(tapes_tip), Some(fjall_tip)) => tapes_tip.as_slice() == fjall_tip,
            _ => false,
        })
    }

    /// Returns Fjall and Tapes read transactions at the same main-chain tip.
    pub fn read_transactions(
        &self,
    ) -> Result<(fjall::Snapshot, TapesReadTransaction), BlockchainError> {
        loop {
            let fjall = self.fjall.snapshot();
            let tapes = self.linear_tapes.reader();

            if self.tips_match(&fjall, &tapes)? {
                return Ok((fjall, tapes));
            }

            // TODO: bound this and panic if we can't get the txs to agree.
        }
    }

    /// Checks if the fjall and tapes database are in sync and rebuilds the fjall database if it
    /// is not.
    pub fn make_consistent(&mut self) -> Result<(), BlockchainError> {
        tracing::info!("Checking blockchain database consistency.");

        if self.config.prune && self.metadata.get_pruning_seed() == PruningSeed::NotPruned {
            // we haven't yet pruned
            self.prune_chain()?;
        }

        let tips_match = {
            let fjall = self.fjall.snapshot();
            let tapes = self.linear_tapes.reader();
            self.tips_match(&fjall, &tapes)?
        };

        if !tips_match {
            tracing::warn!("fjall and tapes are out of sync");
            self.rebuild_fjall_database()?;
        }

        Ok(())
    }

    /// Rebuilds the fjall database.
    pub fn rebuild_fjall_database(&mut self) -> Result<(), BlockchainError> {
        self.block_heights = recreate_fjall_keyspace(&self.fjall, &self.block_heights)?;
        self.chain_tip = recreate_fjall_keyspace(&self.fjall, &self.chain_tip)?;
        self.key_images = recreate_fjall_keyspace(&self.fjall, &self.key_images)?;
        self.pre_rct_outputs = recreate_fjall_keyspace(&self.fjall, &self.pre_rct_outputs)?;
        self.tx_ids = recreate_fjall_keyspace(&self.fjall, &self.tx_ids)?;
        self.v1_tx_outputs = recreate_fjall_keyspace(&self.fjall, &self.v1_tx_outputs)?;
        reset_fjall_keyspace(&self.fjall, &self.alt_chain_infos)?;
        reset_fjall_keyspace(&self.fjall, &self.alt_block_heights)?;
        reset_fjall_keyspace(&self.fjall, &self.alt_block_infos)?;
        reset_fjall_keyspace(&self.fjall, &self.alt_block_blobs)?;
        reset_fjall_keyspace(&self.fjall, &self.alt_transaction_blobs)?;
        reset_fjall_keyspace(&self.fjall, &self.alt_transaction_infos)?;

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

    #[inline]
    pub const fn pruning_seed(&self) -> PruningSeed {
        self.metadata.get_pruning_seed()
    }

    /// - generate new [`PruningSeed`]
    /// - populate [`BlockchainDatabase::prunable_tip`] with latest blocks
    /// - delete unnecessary [`BlockchainDatabase::prunable_blobs`]
    fn prune_chain(&mut self) -> Result<(), BlockchainError> {
        // generate a random stripe index to prune
        let stripe_idx = rand::thread_rng().gen_range(
            1..=u32::try_from(PRUNABLE_BLOBS.len())
                .expect("there shouldn't be that many prunable blobs"),
        );
        self.metadata.set_stripe_idx(stripe_idx)?;

        tracing::info!(
            "initiating pruning on stripe = {:?}.",
            self.metadata
                .get_pruning_seed()
                .get_stripe()
                .expect("should be set now")
        );

        self.populate_pruning_tip()?;
        self.delete_prunable_tapes(u32_to_usize(stripe_idx))
    }

    /// since the tip is currently 5500 blocks wide and each stripe stores currently 4096 blocks, we have at most 3 stripes to look into
    ///
    /// go through each stripe from `chain_height` minus `CRYPTONOTE_PRUNING_STRIPE_SIZE` until the stripe height is above our `chain_height` and:
    /// - read the [`BlockchainDatabase::tx_infos`] for the current stripe
    /// - from this, get [`TxInfo::prunable_blob_idx`] (to get the index) and [`TxInfo::prunable_size`] (to get the length) to read from the [`BlockchainDatabase::prunable_blobs`] for the current stripe
    /// - write the [`TxId`]s and the blobs to the [`BlockchainDatabase::prunable_tip`]
    fn populate_pruning_tip(&self) -> Result<(), BlockchainError> {
        /// Returns the:
        /// - [`BlockHeight`] of the next stripe
        /// - stripe index of the current stripe
        /// - tx id to start reading from
        /// - number of [`BlockchainDatabase::tx_infos`] to read
        fn get_values_to_read(
            db: &BlockchainDatabase,
            chain_height: usize,
            start_height: usize,
            tapes: &impl TapesRead,
        ) -> Result<(BlockHeight, u32, TxId, usize), BlockchainError> {
            // get tx_id to add from
            let start_tx_id = tapes
                .read_entry(&db.block_infos, start_height as u64)?
                .map_or(0, |info| info.mining_tx_index + 1); // skip the miner tx

            // get tx_id to add to
            // for this: current stripe idx -> height for next unpruned block -> get miner tx for this height or if at the end: len of all txs + 1

            // current stripe idx
            let start_stripe_idx = cuprate_pruning::get_block_pruning_stripe(
                start_height,
                usize::MAX,
                CRYPTONOTE_PRUNING_LOG_STRIPES,
            )
            .expect("chain_height - CRYPTONOTE_PRUNING_TIP_BLOCKS <<< usize::MAX");
            // height for next unpruned block
            let next_pruning_seed =
                PruningSeed::new_pruned((start_stripe_idx % 8) + 1, CRYPTONOTE_PRUNING_LOG_STRIPES)
                    .expect("both the stripe_idx and log stripes are valid");
            let next_height = next_pruning_seed
                .get_next_unpruned_block(start_height, MAX_BLOCK_HEIGHT_USIZE)
                .expect("start_height > MAX_BLOCK_HEIGHT");
            // get miner tx for this height or if at the end: len of all txs + 1
            let next_tx_miner_id = if next_height <= chain_height {
                tapes
                    .read_entry(&db.block_infos, next_height as u64)?
                    .map_or(0, |info| info.mining_tx_index)
            } else {
                tapes
                    .fixed_sized_tape_len(&db.tx_infos)
                    .expect("Required tape not found")
                    + 1 // we won't read the last one
            };

            assert!(start_tx_id < next_tx_miner_id); // they can't be equal since the next tx id is a miner id
            let tx_infos_len = u64_to_usize(next_tx_miner_id - 1 - start_tx_id);

            Ok((next_height, start_stripe_idx, start_tx_id, tx_infos_len))
        }

        /// read [`BlockchainDatabase::tx_infos`] and [`BlockchainDatabase::prunable_blobs`]
        /// - read [`BlockchainDatabase::tx_infos`]
        /// - read [`BlockchainDatabase::prunable_blobs`] for current `start_stripe_idx` at `prunable_blob_idx` of first [`TxInfo`] for the sum of all `prunable_size`s
        fn read_tx_infos_and_blob(
            db: &BlockchainDatabase,
            start_tx_id: TxId,
            stripe_idx: u32,
            tx_info_buf: &mut Vec<TxInfo>,
            prunable_blob_buf: &mut Vec<u8>,
            tapes: &impl TapesRead,
        ) -> Result<(), BlockchainError> {
            tapes.read_entries(&db.tx_infos, start_tx_id, tx_info_buf)?;
            // get rid of v1 txs and miner txs
            tx_info_buf.retain(|info| !(info.is_v1_tx() || info.prunable_size == 0));

            let bytes_to_read = tx_info_buf.iter().map(|info| info.prunable_size).sum();
            prunable_blob_buf.resize(bytes_to_read, 0);
            tapes.read_bytes(
                db.prunable_blobs[u32_to_usize(stripe_idx - 1)]
                    .as_ref()
                    .expect("Required tape not found"),
                tx_info_buf[0].prunable_blob_idx,
                prunable_blob_buf,
            )?;

            Ok(())
        }

        fn write_to_prunable_tip(
            db: &BlockchainDatabase,
            tx_info_buf: &[TxInfo],
            start_tx_id: TxId,
            prunable_blob_buf: &[u8],
            w: &mut fjall::OwnedWriteBatch,
        ) {
            let first_prunable_idx = tx_info_buf.first().map_or(0, |info| info.prunable_blob_idx);
            for (tx_id, blob_idx, blob_size) in tx_info_buf.iter().enumerate().map(|(i, info)| {
                (
                    i as u64 + start_tx_id,
                    u64_to_usize(info.prunable_blob_idx - first_prunable_idx),
                    info.prunable_size,
                )
            }) {
                w.insert(
                    &db.prunable_tip,
                    tx_id.to_le_bytes(),
                    &prunable_blob_buf[blob_idx..blob_idx + blob_size],
                );
            }
        }

        let tapes = self.linear_tapes.reader();
        let mut w = self.fjall.batch();

        let chain_height = chain_height(self, &tapes)?;

        // after each stripe, we write to the db
        // allocate all of it first, since we'll most-likely need that much
        let mut tx_info_buf = Vec::new();
        let mut prunable_blob_buf = Vec::new();

        let mut start_height = chain_height.saturating_sub(CRYPTONOTE_PRUNING_TIP_BLOCKS);
        let mut should_break = false;

        loop {
            let (next_height, stripe_idx, start_tx_id, tx_infos_len) =
                get_values_to_read(self, chain_height, start_height, &tapes)?;

            if next_height >= chain_height {
                should_break = true;
            }

            // read the tx infos and blobs
            tx_info_buf.resize(tx_infos_len, TxInfo::default());
            read_tx_infos_and_blob(
                self,
                start_tx_id,
                stripe_idx,
                &mut tx_info_buf,
                &mut prunable_blob_buf,
                &tapes,
            )?;

            // finally, insert first blob + tx
            write_to_prunable_tip(self, &tx_info_buf, start_tx_id, &prunable_blob_buf, &mut w);

            // break if we're at the chain_height
            if should_break {
                break;
            }
            start_height = next_height;
        }

        Ok(())
    }

    fn delete_prunable_tapes(&mut self, stripe_idx: usize) -> Result<(), BlockchainError> {
        let tapes_blob_dir = self.config.blob_dir.join("tapes");
        let prunable_tape_open_options = TapeOpenOptions {
            top_cache_size: self.config.cache_sizes.prunable_blobs,
            dir: tapes_blob_dir,
        };
        for tape_name in (1..=PRUNABLE_BLOBS.len())
            .filter_map(|i| (i != stripe_idx).then_some(PRUNABLE_BLOBS[i - 1]))
        {
            self.linear_tapes
                .delete_tape(tape_name, &prunable_tape_open_options)?;
        }
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
