use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use arc_swap::ArcSwap;
use fjall::{KeyspaceCreateOptions, KvSeparationOptions, PersistMode, Readable};
use monero_oxide::transaction::Transaction;
use rand::Rng;
use tapes::{Persistence, TapeOpenOptions, Tapes, TapesAppend, TapesRead, TapesReadTransaction};

use cuprate_helper::cast::{u32_to_usize, u64_to_usize, usize_to_u64};
use cuprate_pruning::{PruningSeed, CRYPTONOTE_PRUNING_LOG_STRIPES, CRYPTONOTE_PRUNING_TIP_BLOCKS};

use crate::{
    config::Config,
    types::{Amount, BlockInfo, RctOutput, TxInfo},
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

    pub(crate) tapes_metadata: tapes::BlobTape,

    /// Includes the top 5500 blocks, since pruned nodes have to always keep this.
    ///
    /// | key                        | value                  |
    /// |----------------------------|------------------------|
    /// | Tx ID: u64 (little endian) | prunable blob: [u8]    |
    pub(crate) prunable_tip: Option<fjall::Keyspace>,

    /// A runtime cache of the number of outputs for each pre-rct output amount.
    /// This is filled in lazily.
    pub(crate) pre_rct_numb_outputs_cache: Mutex<HashMap<Amount, u64>>,

    pruning_seed: PruningSeed,
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

        // If we already have a `prunable_tip` keyspace then open it here, otherwise we will make a
        // new one and fill it in later if pruning.
        let prunable_tip = fjall
            .keyspace_exists("prunable_tip")
            .then(|| {
                fjall.keyspace("prunable_tip", || {
                    KeyspaceCreateOptions::default().with_kv_separation(Some(
                        KvSeparationOptions::default()
                            .separation_threshold(3_000)
                            .compression(fjall::CompressionType::None),
                    ))
                })
            })
            .transpose()?;

        let tapes_index_dir = config.index_dir.join("tapes");
        let tapes_blob_dir = config.blob_dir.join("tapes");

        let mut linear_tapes = Tapes::open(&tapes_index_dir)?;
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
        let tapes_metadata = tape_append_tx.open_blob_tape(
            "tapes_metadata",
            &TapeOpenOptions {
                top_cache_size: 8,
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

        let pruning_seed = if tape_append_tx.blob_tape_len(&tapes_metadata).unwrap_or(0) == 0 {
            PruningSeed::NotPruned
        } else {
            let mut seed_bytes = [0; 4];
            tape_append_tx.read_bytes(&tapes_metadata, 0, &mut seed_bytes)?;

            PruningSeed::decompress(u32::from_le_bytes(seed_bytes)).unwrap()
        };

        let prunable_blobs = (0..8)
            .map(|i| {
                if pruning_seed
                    .get_stripe()
                    .is_none_or(|stripe| u32_to_usize(stripe) - 1 == i)
                    || prunable_tip.is_none()
                {
                    tape_append_tx
                        .open_blob_tape(PRUNABLE_BLOBS[i], &prunable_tape_open_options)
                        .map(Some)
                } else {
                    Ok(None)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        tape_append_tx.commit(Persistence::SyncAll)?;

        for (i, prunable_blob) in prunable_blobs.iter().enumerate() {
            if prunable_blob.is_none() {
                linear_tapes.delete_tape(PRUNABLE_BLOBS[i], &prunable_tape_open_options)?;
            }
        }

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
            tapes_metadata,
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
            pruning_seed,
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
        let tips_match = {
            let fjall = self.fjall.snapshot();
            let tapes = self.linear_tapes.reader();
            self.tips_match(&fjall, &tapes)?
        };

        if !tips_match {
            tracing::warn!("fjall and tapes are out of sync");
            self.rebuild_fjall_database()?;
        }

        // If we are pruning and don't have the `prunable_tip` table then enable pruning.
        if (self.config.prune || self.pruning_seed != PruningSeed::NotPruned)
            && self.prunable_tip.is_none()
        {
            self.enable_pruning()?;
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

        if let Some(prunable_tip) = self.prunable_tip.take() {
            self.fjall.delete_keyspace(prunable_tip)?;
        }

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

            (Cow::Owned(tx), || Cow::Owned(vec![]))
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
        self.pruning_seed
    }

    /// - generate new [`PruningSeed`]
    /// - populate [`BlockchainDatabase::prunable_tip`] with latest blocks
    /// - delete unnecessary [`BlockchainDatabase::prunable_blobs`]
    fn enable_pruning(&mut self) -> Result<(), BlockchainError> {
        if self.pruning_seed == PruningSeed::NotPruned {
            let mut tapes_tx = self.linear_tapes.append();

            // generate a random stripe index to prune
            let stripe_idx = rand::thread_rng().gen_range(
                1..=u32::try_from(PRUNABLE_BLOBS.len())
                    .expect("there shouldn't be that many prunable blobs"),
            );
            let seed = PruningSeed::new_pruned(stripe_idx, CRYPTONOTE_PRUNING_LOG_STRIPES).unwrap();
            self.pruning_seed = seed;
            tapes_tx.append_bytes(&self.tapes_metadata, &seed.compress().to_le_bytes())?;
            tapes_tx.commit(Persistence::SyncAll)?;
        }

        tracing::info!(
            "Pruning chain on stripe = {:?}.",
            self.pruning_seed.get_stripe().unwrap()
        );

        let tapes_reader = self.linear_tapes.reader();
        let mut w = self.fjall.batch();

        let prunable_tip = self.fjall.keyspace("prunable_tip", || {
            KeyspaceCreateOptions::default().with_kv_separation(Some(
                KvSeparationOptions::default()
                    .separation_threshold(3_000)
                    .compression(fjall::CompressionType::None),
            ))
        })?;

        let start_tip_height = tapes_reader
            .fixed_sized_tape_len(&self.block_infos)
            .unwrap_or(0)
            .saturating_sub(usize_to_u64(CRYPTONOTE_PRUNING_TIP_BLOCKS));
        let start_tx_idx = tapes_reader
            .read_entry(&self.block_infos, start_tip_height)?
            .map_or(0, |info| info.mining_tx_index);
        let end_tx_idx = tapes_reader
            .fixed_sized_tape_len(&self.tx_infos)
            .unwrap_or(0);

        for (i, tx_id) in (start_tx_idx..end_tx_idx).rev().enumerate() {
            let tx_info = tapes_reader.read_entry(&self.tx_infos, tx_id)?.unwrap();

            if tx_info.is_v1_tx() {
                continue;
            }
            let stripe = cuprate_pruning::get_block_pruning_stripe(
                tx_info.height,
                usize::MAX,
                CRYPTONOTE_PRUNING_LOG_STRIPES,
            )
            .unwrap();

            let Some(prunable_blob) = self.prunable_blobs
                [usize::try_from(stripe).expect("stripe will not exceed usize::MAX") - 1]
                .as_ref()
            else {
                self.prunable_tip = Some(prunable_tip);
                w.commit()?;

                return Ok(());
            };

            let mut blob = vec![0; tx_info.prunable_size];
            tapes_reader.read_bytes(prunable_blob, tx_info.prunable_blob_idx, &mut blob)?;

            w.insert(&prunable_tip, tx_id.to_le_bytes(), blob.as_slice());

            if (i + 1) % 1000 == 0 {
                w.commit()?;
                w = self.fjall.batch();
            }
        }
        self.prunable_tip = Some(prunable_tip);
        w.commit()?;

        let tapes_blob_dir = self.config.blob_dir.join("tapes");
        let prunable_tape_open_options = TapeOpenOptions {
            top_cache_size: self.config.cache_sizes.prunable_blobs,
            dir: tapes_blob_dir,
        };

        drop(tapes_reader);
        let stripe = self.pruning_seed.get_stripe().unwrap();
        for (i, prunable_blob) in self.prunable_blobs.iter_mut().enumerate() {
            if u32_to_usize(stripe) - 1 != i {
                self.linear_tapes
                    .delete_tape(PRUNABLE_BLOBS[i], &prunable_tape_open_options)?;
                *prunable_blob = None;
            }
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
