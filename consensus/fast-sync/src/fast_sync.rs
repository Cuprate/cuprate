use blake3::Hasher;
use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_consensus_context::BlockchainContext;
use cuprate_p2p::block_downloader::ChainEntry;
use cuprate_p2p_core::NetworkZone;
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};
use cuprate_types::{Chain, VerifiedBlockInformation, VerifiedTransactionInformation};
use monero_serai::block::Block;
use monero_serai::transaction::{Input, Transaction};
use std::cmp::min;
use std::collections::{HashMap, VecDeque};
use std::slice;
use tower::{Service, ServiceExt};

static FAST_SYNC_HASHES: &[[u8; 32]] = unsafe {
    let bytes = include_bytes!("./data/fast_sync_hashes.bin");
    if bytes.len() % 32 != 0 {
        panic!()
    }

    slice::from_raw_parts(bytes.as_ptr().cast::<[u8; 32]>(), bytes.len() / 32)
};

const FAST_SYNC_BATCH_LEN: usize = 512;

pub static FAST_SYNC_TOP_HEIGHT: usize = FAST_SYNC_HASHES.len() * FAST_SYNC_BATCH_LEN;

pub async fn validate_entries<N: NetworkZone>(
    mut entries: VecDeque<ChainEntry<N>>,
    start_height: usize,
    blockchain_read_handle: &mut BlockchainReadHandle,
) -> Result<(VecDeque<ChainEntry<N>>, VecDeque<ChainEntry<N>>), tower::BoxError> {
    if start_height > FAST_SYNC_TOP_HEIGHT {
        return Ok((entries, VecDeque::new()));
    }

    let hashes_start_height = (start_height / FAST_SYNC_BATCH_LEN) * FAST_SYNC_BATCH_LEN;
    let amount_of_hashes = entries.iter().map(|e| e.ids.len()).sum::<usize>();
    let last_height = amount_of_hashes + start_height;

    let hashes_stop_height = min(
        (last_height / FAST_SYNC_BATCH_LEN) * FAST_SYNC_BATCH_LEN,
        FAST_SYNC_TOP_HEIGHT,
    );

    let mut hashes_stop_diff_last_height = last_height - hashes_stop_height;

    let mut unknown = VecDeque::new();

    while !entries.is_empty() && hashes_stop_diff_last_height != 0 {
        let back = entries.back_mut().unwrap();

        if back.ids.len() >= hashes_stop_diff_last_height {
            unknown.push_front(ChainEntry {
                ids: back
                    .ids
                    .drain((back.ids.len() - hashes_stop_diff_last_height)..)
                    .collect(),
                peer: back.peer.clone(),
                handle: back.handle.clone(),
            });

            break;
        }

        let back = entries.pop_back().unwrap();
        hashes_stop_diff_last_height -= back.ids.len();
        unknown.push_front(back);
    }

    let BlockchainResponse::BlockHashInRange(hashes) = blockchain_read_handle
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockHashInRange(
            hashes_start_height..start_height,
            Chain::Main
        ))
        .await?
    else {
        unreachable!()
    };

    let mut hasher = Hasher::default();
    for (i, hash) in hashes
        .iter()
        .chain(entries.iter().flat_map(|e| e.ids.iter()))
        .enumerate()
    {
        hasher.update(hash);

        if (i + 1) % FAST_SYNC_BATCH_LEN == 0 {
            let got_hash = hasher.finalize();

            if got_hash != FAST_SYNC_HASHES[get_hash_index_for_height(hashes_start_height + i)] {
                return Err("Hashes do not match".into());
            }
            hasher.reset();
        }
    }

    Ok((entries, unknown))
}

fn get_hash_index_for_height(height: usize) -> usize {
    height / FAST_SYNC_BATCH_LEN
}

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

    if *height != blockchin_ctx.chain_height {
        panic!("fast sync block invalid");
    }

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
