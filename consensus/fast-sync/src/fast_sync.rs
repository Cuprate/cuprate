use std::{
    cmp::min,
    collections::{HashMap, VecDeque},
    sync::OnceLock,
};

use blake3::Hasher;
use monero_serai::{
    block::Block,
    transaction::{Input, Transaction},
};
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_consensus_context::BlockchainContext;
use cuprate_p2p::block_downloader::ChainEntry;
use cuprate_p2p_core::NetworkZone;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain, VerifiedBlockInformation, VerifiedTransactionInformation,
};

/// A [`OnceLock`] representing the fast sync hashes.
static FAST_SYNC_HASHES: OnceLock<&[[u8; 32]]> = OnceLock::new();

/// The size of a batch of block hashes to hash to create a fast sync hash.
pub const FAST_SYNC_BATCH_LEN: usize = 512;

/// Returns the height of the first block not included in the embedded hashes.
///
/// # Panics
///
/// This function will panic if [`set_fast_sync_hashes`] has not been called.
pub fn fast_sync_stop_height() -> usize {
    FAST_SYNC_HASHES.get().unwrap().len() * FAST_SYNC_BATCH_LEN
}

/// Sets the hashes to use for fast-sync.
///
/// # Panics
///
/// This will panic if this is called more than once.
pub fn set_fast_sync_hashes(hashes: &'static [[u8; 32]]) {
    FAST_SYNC_HASHES.set(hashes).unwrap();
}

/// Validates that the given [`ChainEntry`]s are in the fast-sync hashes.
///
/// `entries` should be a list of sequential entries.
/// `start_height` should be the height of the first block in the first entry.
///
/// Returns a tuple, the first element being the entries that are valid* the second
/// the entries we do not know are valid and should be passed in again when we have more entries.
///
/// *once we are passed the fast sync blocks all entries will be returned as valid as
/// we can not check their validity here.
///
/// There may be more entries returned than passed in as entries could be split.
///
/// # Panics
///
/// This will panic if [`set_fast_sync_hashes`] has not been called.
pub async fn validate_entries<N: NetworkZone>(
    mut entries: VecDeque<ChainEntry<N>>,
    start_height: usize,
    blockchain_read_handle: &mut BlockchainReadHandle,
) -> Result<(VecDeque<ChainEntry<N>>, VecDeque<ChainEntry<N>>), tower::BoxError> {
    // if we are past the top fast sync block return all entries as valid.
    if start_height >= fast_sync_stop_height() {
        return Ok((entries, VecDeque::new()));
    }

    /*
       The algorithm used here needs to preserve which peer told us about which blocks, so we cannot
       simply join all the hashes together return all the ones that can be validated and the ones that
       can't, we need to keep the batches separate.

       The first step is to calculate how many hashes we need from the blockchain to make up the first
       fast-sync hash.

       Then will take out all the batches at the end for which we cannot make up a full fast-sync hash
       for, we will split a batch if it can only be partially validated.

       With the remaining hashes from the blockchain and the hashes in the batches we can validate we
       work on calculating the fast sync hashes and comparing them to the ones in [`FAST_SYNC_HASHES`].
    */

    // First calculate the start and stop for this range of hashes.
    let hashes_start_height = (start_height / FAST_SYNC_BATCH_LEN) * FAST_SYNC_BATCH_LEN;
    let amount_of_hashes = entries.iter().map(|e| e.ids.len()).sum::<usize>();
    let last_height = amount_of_hashes + start_height;

    let hashes_stop_height = min(
        (last_height / FAST_SYNC_BATCH_LEN) * FAST_SYNC_BATCH_LEN,
        fast_sync_stop_height(),
    );

    let mut hashes_stop_diff_last_height = last_height - hashes_stop_height;

    // get the hashes we are missing to create the first fast-sync hash.
    let BlockchainResponse::BlockHashInRange(starting_hashes) = blockchain_read_handle
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockHashInRange(
            hashes_start_height..start_height,
            Chain::Main,
        ))
        .await?
    else {
        unreachable!()
    };

    // If we don't have enough hashes to make up a batch we can't validate any.
    if amount_of_hashes + starting_hashes.len() < FAST_SYNC_BATCH_LEN {
        return Ok((VecDeque::new(), entries));
    }

    let mut unknown = VecDeque::new();

    // start moving from the back of the batches taking enough hashes out so we are only left with hashes
    // that can be verified.
    while !entries.is_empty() && hashes_stop_diff_last_height != 0 {
        let back = entries.back_mut().unwrap();

        if back.ids.len() >= hashes_stop_diff_last_height {
            // This batch is partially valid so split it.
            unknown.push_front(ChainEntry {
                ids: back
                    .ids
                    .drain((back.ids.len() - hashes_stop_diff_last_height)..)
                    .collect(),
                peer: back.peer,
                handle: back.handle.clone(),
            });

            break;
        }

        // Add this batch to the front of the unknowns, we do not know its validity.
        let back = entries.pop_back().unwrap();
        hashes_stop_diff_last_height -= back.ids.len();
        unknown.push_front(back);
    }

    // Start verifying the hashes.
    let mut hasher = Hasher::default();
    let mut last_i = 1;
    for (i, hash) in starting_hashes
        .iter()
        .chain(entries.iter().flat_map(|e| e.ids.iter()))
        .enumerate()
    {
        hasher.update(hash);

        if (i + 1) % FAST_SYNC_BATCH_LEN == 0 {
            let got_hash = hasher.finalize();

            if got_hash
                != FAST_SYNC_HASHES.get().unwrap()
                    [get_hash_index_for_height(hashes_start_height + i)]
            {
                return Err("Hashes do not match".into());
            }
            hasher.reset();
        }

        last_i = i + 1;
    }
    // Make sure we actually checked all hashes.
    assert_eq!(last_i % FAST_SYNC_BATCH_LEN, 0);

    Ok((entries, unknown))
}

/// Get the index of the hash that contains this block in the fast sync hashes.
const fn get_hash_index_for_height(height: usize) -> usize {
    height / FAST_SYNC_BATCH_LEN
}

/// Creates a [`VerifiedBlockInformation`] from a block known to be valid.
///
/// # Panics
///
/// This may panic if used on an invalid block.
pub fn block_to_verified_block_information(
    block: Block,
    txs: Vec<Transaction>,
    blockchin_ctx: &BlockchainContext,
) -> VerifiedBlockInformation {
    let block_hash = block.hash();

    let block_blob = block.serialize();

    let Some(Input::Gen(height)) = block.miner_transaction.prefix().inputs.first() else {
        panic!("fast sync block invalid");
    };

    assert_eq!(
        *height, blockchin_ctx.chain_height,
        "fast sync block invalid"
    );

    let mut txs = txs
        .into_iter()
        .map(|tx| {
            let data = new_tx_verification_data(tx).expect("fast sync block invalid");

            (data.tx_hash, data)
        })
        .collect::<HashMap<_, _>>();

    let mut verified_txs = Vec::with_capacity(txs.len());
    for tx in &block.transactions {
        let data = txs.remove(tx).expect("fast sync block invalid");

        verified_txs.push(VerifiedTransactionInformation {
            tx_blob: data.tx_blob,
            tx_weight: data.tx_weight,
            fee: data.fee,
            tx_hash: data.tx_hash,
            tx: data.tx,
        });
    }

    let total_fees = verified_txs.iter().map(|tx| tx.fee).sum::<u64>();
    let total_outputs = block
        .miner_transaction
        .prefix()
        .outputs
        .iter()
        .map(|output| output.amount.unwrap_or(0))
        .sum::<u64>();

    let generated_coins = total_outputs - total_fees;

    let weight = block.miner_transaction.weight()
        + verified_txs.iter().map(|tx| tx.tx_weight).sum::<usize>();

    VerifiedBlockInformation {
        block_blob,
        txs: verified_txs,
        block_hash,
        pow_hash: [u8::MAX; 32],
        height: *height,
        generated_coins,
        weight,
        long_term_weight: blockchin_ctx.next_block_long_term_weight(weight),
        cumulative_difficulty: blockchin_ctx.cumulative_difficulty + blockchin_ctx.next_difficulty,
        block,
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, slice, sync::LazyLock};

    use proptest::proptest;

    use cuprate_p2p::block_downloader::ChainEntry;
    use cuprate_p2p_core::{client::InternalPeerID, handles::HandleBuilder, ClearNet};

    use crate::{
        fast_sync_stop_height, set_fast_sync_hashes, validate_entries, FAST_SYNC_BATCH_LEN,
    };

    static HASHES: LazyLock<&[[u8; 32]]> = LazyLock::new(|| {
        let hashes = (0..FAST_SYNC_BATCH_LEN * 2000)
            .map(|i| {
                let mut ret = [0; 32];
                ret[..8].copy_from_slice(&i.to_le_bytes());
                ret
            })
            .collect::<Vec<_>>();

        let hashes = hashes.leak();

        let fast_sync_hashes = hashes
            .chunks(FAST_SYNC_BATCH_LEN)
            .map(|chunk| {
                let len = chunk.len() * 32;
                let bytes = chunk.as_ptr().cast::<u8>();

                // SAFETY:
                // We are casting a valid [[u8; 32]] to a [u8], no alignment requirements and we are using it
                // within the [[u8; 32]]'s lifetime.
                unsafe { blake3::hash(slice::from_raw_parts(bytes, len)).into() }
            })
            .collect::<Vec<_>>();

        set_fast_sync_hashes(fast_sync_hashes.leak());

        hashes
    });

    proptest! {
        #[test]
        fn valid_entry(len in 0_usize..1_500_000) {
            let mut ids = HASHES.to_vec();
            ids.resize(len, [0_u8; 32]);

            let handle = HandleBuilder::new().build();

            let entry = ChainEntry {
                ids,
                peer: InternalPeerID::Unknown([1; 16]),
                handle: handle.1
            };

            let data_dir = tempfile::tempdir().unwrap();

            tokio_test::block_on(async move {
                let blockchain_config = cuprate_blockchain::config::ConfigBuilder::new()
                    .data_directory(data_dir.path().to_path_buf())
                    .build();

                let (mut blockchain_read_handle, _, _) =
                    cuprate_blockchain::service::init(blockchain_config).unwrap();


                let ret = validate_entries::<ClearNet>(VecDeque::from([entry]), 0, &mut blockchain_read_handle).await.unwrap();

                let len_left = ret.0.iter().map(|e| e.ids.len()).sum::<usize>();
                let len_right = ret.1.iter().map(|e| e.ids.len()).sum::<usize>();

                assert_eq!(len_left + len_right, len);
                assert!(len_left <= fast_sync_stop_height());
                assert!(len_right < FAST_SYNC_BATCH_LEN || len > fast_sync_stop_height());
            });
        }

        #[test]
        fn single_hash_entries(len in 0_usize..1_500_000) {
            let handle = HandleBuilder::new().build();
            let entries = (0..len).map(|i| {
                ChainEntry {
                    ids: vec![HASHES.get(i).copied().unwrap_or_default()],
                    peer: InternalPeerID::Unknown([1; 16]),
                    handle: handle.1.clone()
                }
            }).collect();

            let data_dir = tempfile::tempdir().unwrap();

            tokio_test::block_on(async move {
                let blockchain_config = cuprate_blockchain::config::ConfigBuilder::new()
                    .data_directory(data_dir.path().to_path_buf())
                    .build();

                let (mut blockchain_read_handle, _, _) =
                    cuprate_blockchain::service::init(blockchain_config).unwrap();


                let ret = validate_entries::<ClearNet>(entries, 0, &mut blockchain_read_handle).await.unwrap();

                let len_left = ret.0.iter().map(|e| e.ids.len()).sum::<usize>();
                let len_right = ret.1.iter().map(|e| e.ids.len()).sum::<usize>();

                assert_eq!(len_left + len_right, len);
                assert!(len_left <= fast_sync_stop_height());
                assert!(len_right < FAST_SYNC_BATCH_LEN || len > fast_sync_stop_height());
            });
        }

        #[test]
        fn not_enough_hashes(len in 0_usize..FAST_SYNC_BATCH_LEN) {
            let hashes_start_height = FAST_SYNC_BATCH_LEN * 1234;

            let handle = HandleBuilder::new().build();
            let entry = ChainEntry {
                ids: HASHES[hashes_start_height..(hashes_start_height + len)].to_vec(),
                peer: InternalPeerID::Unknown([1; 16]),
                handle: handle.1
            };

            let data_dir = tempfile::tempdir().unwrap();

            tokio_test::block_on(async move {
                let blockchain_config = cuprate_blockchain::config::ConfigBuilder::new()
                    .data_directory(data_dir.path().to_path_buf())
                    .build();

                let (mut blockchain_read_handle, _, _) =
                    cuprate_blockchain::service::init(blockchain_config).unwrap();


                let ret = validate_entries::<ClearNet>(VecDeque::from([entry]), 0, &mut blockchain_read_handle).await.unwrap();

                let len_left = ret.0.iter().map(|e| e.ids.len()).sum::<usize>();
                let len_right = ret.1.iter().map(|e| e.ids.len()).sum::<usize>();

                assert_eq!(len_right, len);
                assert_eq!(len_left, 0);
            });
        }
    }
}
