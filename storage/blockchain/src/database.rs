use std::{
    borrow::Cow,
    collections::HashMap,
    mem,
    sync::{Arc, Mutex},
};

use arc_swap::ArcSwap;
use fjall::{KeyspaceCreateOptions, PersistMode, Readable};
use monero_oxide::transaction::Transaction;
use rand::Rng;
use tapes::{
    CachedBlobTape, CachedTapeOpenOptions, FixedSizedTape, Persistence, RollingBlobTape,
    RollingTapeOpenOptions, Tapes, TapesAppend, TapesAppendTransaction, TapesRead,
    TapesReadTransaction, WholeBlobTape, WholeTapeOpenOptions,
};

use cuprate_helper::cast::{u32_to_usize, u64_to_usize, usize_to_u64};
use cuprate_pruning::{PruningSeed, CRYPTONOTE_PRUNING_LOG_STRIPES, CRYPTONOTE_PRUNING_TIP_BLOCKS};

use crate::{
    config::Config,
    types::{Amount, BlockInfo, RctOutput, TxInfo},
    BlockchainError,
};

/// The key used to store the main-chain tip in [`BlockchainDatabase::chain_tip`].
pub(crate) const CHAIN_TIP_KEY: &[u8] = b"tip";

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

const PRUNABLE_TIP_FILE_SIZE: u64 = 4 * 1024 * 1024;
const PRUNABLE_BLOB_FILE_SIZE: u64 = 128 * 1024 * 1024;

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
    pub(crate) rct_outputs: FixedSizedTape<RctOutput, CachedBlobTape<WholeBlobTape>>,
    /// Transaction info, indexed by [`TxId`].
    ///
    /// | index      | value      |
    /// |------------|------------|
    /// | Tx ID: u64 | [`TxInfo`] |
    pub(crate) tx_infos: FixedSizedTape<TxInfo, CachedBlobTape<WholeBlobTape>>,
    /// Block info, indexed by block height.
    ///
    /// | index             | value         |
    /// |-------------------|---------------|
    /// | Block height: u64 | [`BlockInfo`] |
    pub(crate) block_infos: FixedSizedTape<BlockInfo, CachedBlobTape<WholeBlobTape>>,
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
    pub(crate) pruned_blobs: CachedBlobTape<WholeBlobTape>,
    /// V1 prunable transaction blobs, indexed by [`TxInfo::prunable_blob_idx`].
    ///
    /// This tape stores the prunable blob for all V1 txs, these can't be pruned.
    pub(crate) v1_prunable_blobs: CachedBlobTape<WholeBlobTape>,
    /// The pruning tables of this database.
    pub(crate) prunable_tables: PrunableTables,

    /// A runtime cache of the number of outputs for each pre-rct output amount.
    /// This is filled in lazily.
    pub(crate) pre_rct_numb_outputs_cache: Mutex<HashMap<Amount, u64>>,
}

/// The pruning state of the database.
pub(crate) enum PrunableTables {
    /// An unpruned database.
    ///
    /// These tapes store the prunable part of each tx, the stripe a tx is stored in depends on the
    /// height of the block.
    Full(Vec<CachedBlobTape<WholeBlobTape>>),
    /// A pruned database.
    Pruned {
        /// The stripe we keep.
        stripe: u32,
        /// The kept stripe tape.
        kept_stripe: CachedBlobTape<WholeBlobTape>,
        /// Prunable tip index, indexed by [`TxId`]
        ///
        /// # Warning
        ///
        /// Only transactions in pruned blocks (`block_stripe` != `stripe`) will have a valid value
        /// in this table. Other txs will be here but their value is unspecified.
        ///
        /// | index | value                                   |
        /// |-------|------------------------------------------|
        /// | Tx ID | An index (u64) into `prunable_tip_blobs` |
        prunable_tip: FixedSizedTape<u64, CachedBlobTape<RollingBlobTape>>,
        /// The prunable blobs of txs in blocks that will be pruned.
        prunable_tip_blobs: CachedBlobTape<RollingBlobTape>,
    },
}

impl PrunableTables {
    /// Attempt to get the full blob tape for the given stripe, returning [`None`] if
    /// we pruned it.
    pub(crate) fn try_get_prunable_tape(
        &self,
        stripe: u32,
    ) -> Option<&CachedBlobTape<WholeBlobTape>> {
        match self {
            Self::Full(vec) => vec.get(stripe as usize - 1),
            Self::Pruned {
                stripe: our_stripe,
                kept_stripe,
                ..
            } if *our_stripe == stripe => Some(kept_stripe),
            Self::Pruned { .. } => None,
        }
    }

    /// Opens a [`PrunableTables`] using the given tapes transaction.
    fn open(
        tape_append_tx: &mut TapesAppendTransaction,
        config: &Config,
    ) -> Result<Self, BlockchainError> {
        // `prunable_tip` is a tape that is only created when pruned.
        let is_pruned = tape_append_tx.tape_exists("prunable_tip");

        let prunable_tape_open_options = CachedTapeOpenOptions {
            top_cache_size: config.cache_sizes.prunable_blobs,
            inner: WholeTapeOpenOptions {
                dir: config.blob_dir.clone(),
            },
        };

        if is_pruned {
            // Only the stripe we keep will be in the tapes' database.
            // TODO: the tape file of a deleted tape could be left over eating up space in a crash though.
            // Add a way to check and delete tape files so we can make sure they are removed here.
            let kept_stripe = (0..8).find_map(|i| {
                if tape_append_tx.tape_exists(PRUNABLE_BLOBS[i]) {
                    Some((
                        i + 1,
                        tape_append_tx
                            .open_blob_tape(PRUNABLE_BLOBS[i], prunable_tape_open_options.clone()),
                    ))
                } else {
                    None
                }
            });

            let Some((stripe, kept_stripe)) = kept_stripe else {
                return Err(BlockchainError::NotFound);
            };
            let kept_stripe = kept_stripe?;

            // Open the tip tapes. We can use `start_index: 0` here as the tapes database will
            // only take that value into account when creating a new tape. Here we are always opening
            // an already existing tape.
            let prunable_tip_blobs = tape_append_tx.open_blob_tape(
                "prunable_tip_blobs",
                CachedTapeOpenOptions {
                    inner: RollingTapeOpenOptions {
                        file_size: PRUNABLE_BLOB_FILE_SIZE,
                        dir: config.blob_dir.clone(),
                        start_index: 0,
                    },
                    top_cache_size: config.cache_sizes.prunable_blobs,
                },
            )?;

            let prunable_tip = tape_append_tx.open_fixed_sized_tape(
                "prunable_tip",
                CachedTapeOpenOptions {
                    inner: RollingTapeOpenOptions {
                        file_size: PRUNABLE_TIP_FILE_SIZE,
                        dir: config.index_dir.clone(),
                        start_index: 0,
                    },
                    top_cache_size: config.cache_sizes.prunable_tip,
                },
            )?;

            Ok(Self::Pruned {
                stripe: stripe
                    .try_into()
                    .expect("Pruning stripe is in range as we just created it"),
                kept_stripe,
                prunable_tip,
                prunable_tip_blobs,
            })
        } else {
            Ok(Self::Full(
                (0..8)
                    .map(|i| {
                        tape_append_tx
                            .open_blob_tape(PRUNABLE_BLOBS[i], prunable_tape_open_options.clone())
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
    }
}

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

        let linear_tapes = Tapes::open(&config.index_dir)?;
        let mut tape_append_tx = linear_tapes.append();

        let rct_outputs = tape_append_tx.open_fixed_sized_tape(
            "rct_outputs",
            CachedTapeOpenOptions {
                top_cache_size: config.cache_sizes.rct_outputs,
                inner: WholeTapeOpenOptions {
                    dir: config.index_dir.clone(),
                },
            },
        )?;
        let tx_infos = tape_append_tx.open_fixed_sized_tape(
            "tx_infos",
            CachedTapeOpenOptions {
                top_cache_size: config.cache_sizes.tx_infos,
                inner: WholeTapeOpenOptions {
                    dir: config.index_dir.clone(),
                },
            },
        )?;
        let block_infos = tape_append_tx.open_fixed_sized_tape(
            "block_infos",
            CachedTapeOpenOptions {
                top_cache_size: config.cache_sizes.block_infos,
                inner: WholeTapeOpenOptions {
                    dir: config.index_dir.clone(),
                },
            },
        )?;

        let pruned_blobs = tape_append_tx.open_blob_tape(
            "pruned_blobs",
            CachedTapeOpenOptions {
                top_cache_size: config.cache_sizes.pruned_blobs,
                inner: WholeTapeOpenOptions {
                    dir: config.blob_dir.clone(),
                },
            },
        )?;
        let v1_prunable_blobs = tape_append_tx.open_blob_tape(
            "v1_prunable_blobs",
            CachedTapeOpenOptions {
                top_cache_size: config.cache_sizes.v1_prunable_blobs,
                inner: WholeTapeOpenOptions {
                    dir: config.blob_dir.clone(),
                },
            },
        )?;

        let prunable_tables = PrunableTables::open(&mut tape_append_tx, config)?;

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
            prunable_tables,
            pre_rct_numb_outputs_cache: Mutex::new(HashMap::new()),
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

        // If we are pruning and have not yet pruned then prune.
        if self.config.prune && matches!(self.prunable_tables, PrunableTables::Full(_)) {
            self.enable_pruning()?;
        }

        Ok(())
    }

    /// Rebuilds the fjall database.
    ///
    /// This will not fill in the prunable tip blocks.
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
                let old_batch = mem::replace(
                    &mut batch,
                    self.fjall.batch().durability(Some(PersistMode::Buffer)),
                );

                old_batch.commit()?;
            }
        }

        batch.commit()?;

        Ok(())
    }

    /// Returns the [`PruningSeed`] for this database.
    #[inline]
    pub fn pruning_seed(&self) -> PruningSeed {
        match &self.prunable_tables {
            PrunableTables::Full(_) => PruningSeed::NotPruned,
            PrunableTables::Pruned { stripe, .. } => {
                PruningSeed::new_pruned(*stripe, CRYPTONOTE_PRUNING_LOG_STRIPES).unwrap()
            }
        }
    }

    /// - generate new [`PruningSeed`] (if one doesn't exist)
    /// - populate [`BlockchainDatabase::prunable_tip`] with latest blocks
    /// - delete unnecessary [`BlockchainDatabase::prunable_blobs`]
    fn enable_pruning(&mut self) -> Result<(), BlockchainError> {
        // generate a random stripe index to prune
        let stripe = rand::thread_rng().gen_range(
            1..=u32::try_from(PRUNABLE_BLOBS.len())
                .expect("there shouldn't be that many prunable blobs"),
        );

        // Take the prunable tables, we will set it again before returning Ok.
        let PrunableTables::Full(prunable_blobs) =
            mem::replace(&mut self.prunable_tables, PrunableTables::Full(vec![]))
        else {
            unreachable!("Database is already pruned");
        };

        tracing::info!("Pruning chain on stripe = {:?}.", stripe);

        // This transaction is the only transaction for the whole pruning process, it will atomically
        // prune the DB.
        let mut tapes_append = self.linear_tapes.append();

        let start_tip_height = tapes_append
            .fixed_sized_tape_len(&self.block_infos)
            .unwrap_or(0)
            .saturating_sub(usize_to_u64(CRYPTONOTE_PRUNING_TIP_BLOCKS));
        let start_tx_idx = tapes_append
            .read_entry(&self.block_infos, start_tip_height)?
            .map_or(0, |info| info.mining_tx_index);
        let end_tx_idx = tapes_append
            .fixed_sized_tape_len(&self.tx_infos)
            .unwrap_or(0);

        let prunable_tip_blobs = tapes_append.open_blob_tape(
            "prunable_tip_blobs",
            CachedTapeOpenOptions {
                inner: RollingTapeOpenOptions {
                    file_size: PRUNABLE_BLOB_FILE_SIZE,
                    dir: self.config.blob_dir.clone(),
                    start_index: 0,
                },
                top_cache_size: self.config.cache_sizes.prunable_blobs,
            },
        )?;

        let prunable_tip: FixedSizedTape<_, CachedBlobTape<RollingBlobTape>> = tapes_append
            .open_fixed_sized_tape(
                "prunable_tip",
                CachedTapeOpenOptions {
                    inner: RollingTapeOpenOptions {
                        file_size: PRUNABLE_TIP_FILE_SIZE,
                        dir: self.config.index_dir.clone(),
                        // We want to start the indexing at the tx index of the first tip tx.
                        // start_index is measured in raw bytes.
                        start_index: start_tx_idx * usize_to_u64(size_of::<u64>()),
                    },
                    top_cache_size: self.config.cache_sizes.prunable_tip,
                },
            )?;

        // fill in the tip tapes.
        for tx_id in start_tx_idx..end_tx_idx {
            let tx_info = tapes_append.read_entry(&self.tx_infos, tx_id)?.unwrap();

            let block_stripe = cuprate_pruning::get_block_pruning_stripe(
                tx_info.height,
                usize::MAX,
                CRYPTONOTE_PRUNING_LOG_STRIPES,
            )
            .unwrap();

            let prunable_blob = &prunable_blobs
                [usize::try_from(block_stripe).expect("stripe will not exceed usize::MAX") - 1];

            // V1 txs are always unpruned, but we still need to store them in `prunable_tip` to keep the
            // index tracking of that tape correct.
            let blob = if tx_info.is_v1_tx() {
                vec![]
            } else {
                let mut b = vec![0; tx_info.prunable_size];
                tapes_append.read_bytes(prunable_blob, tx_info.prunable_blob_idx, &mut b)?;
                b
            };

            let idx = tapes_append.append_bytes(&prunable_tip_blobs, &blob)?;

            let tx_id_2 = tapes_append.append_entries(&prunable_tip, &[idx])?;
            // Make sure the index is what we expect.
            assert_eq!(tx_id_2, tx_id);
        }

        // Delete the tapes we no longer need.
        let mut kept_stripe = None;
        for (i, prunable_blob) in prunable_blobs.into_iter().enumerate() {
            if u32_to_usize(stripe) - 1 == i {
                kept_stripe = Some(prunable_blob);
            } else {
                tapes_append.delete_tape(prunable_blob);
            }
        }

        // Prune the DB!
        tapes_append.commit(Persistence::SyncAll)?;

        self.prunable_tables = PrunableTables::Pruned {
            stripe,
            prunable_tip_blobs,
            prunable_tip,
            kept_stripe: kept_stripe.unwrap(),
        };

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
