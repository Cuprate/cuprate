//! Handles creating a block-template with all the relevant methods.
//!
//! This mainly includes:
//! - selecting txs from the pool
//! - constructing a miner transaction
//! - finding a nonce, for `generateblocks`
//!
//! Known differences from monerod:
//! - Templates are not cached, so every call returns a new transaction key and blob.
//! - Pool transactions are only checked for spent key images. One that a reorg or
//!   `pop_blocks` invalidated stays in templates, which are then rejected, until it leaves the pool.
//! - The parent is always the top block, so alternative chains cannot be extended.
//! - `expected_reward` is exactly what the miner transaction pays. Above the median
//!   weight, monerod's is an estimate that ignores the miner transaction's own weight.
use std::collections::HashSet;

use anyhow::{anyhow, ensure, Error};
use bytes::BufMut;
use monero_address::MoneroAddress;
use monero_oxide::{
    block::{Block, BlockHeader},
    ed25519::{CompressedPoint, Point, Scalar},
    io::VarInt,
    primitives::keccak256,
    transaction::{Input, Output, Timelock, Transaction, TransactionPrefix},
};
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_blockchain::{service::BlockchainReadHandle, types::BlockHash};
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockchainContextService,
    BlockchainReadRequest, BlockchainResponse,
};
use cuprate_consensus_rules::{
    blocks::{
        calculate_pow_hash, check_block_pow, check_block_weight, randomx_seed_height,
        ContextToVerifyBlock, RX_SEEDHASH_EPOCH_LAG,
    },
    miner_tx::{calculate_block_reward, MINER_TX_TIME_LOCKED_BLOCKS},
    transactions::CRYPTONOTE_COINBASE_BLOB_RESERVED_SIZE,
};
use cuprate_helper::{asynch::rayon_spawn_async, cast::usize_to_u64, time::current_unix_timestamp};
use cuprate_txpool::{service::TxpoolReadHandle, types::TransactionHash, TxEntry};
use cuprate_types::{BlockTemplate, HardFork};

use crate::txpool::read::{backlog, txs_for_block};

const ACCEPT_THRESHOLD: u64 = 1;
/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_basic/tx_extra.h#L35-L39>
pub(crate) const TX_EXTRA_NONCE_MAX_COUNT: usize = 255;
const TX_EXTRA_TAG_PUBKEY: u8 = 0x01;
const TX_EXTRA_NONCE: u8 = 0x02;
/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_config.h#L220-L221>
const DEFAULT_DUST_THRESHOLD: u64 = 2000000000; // 2 * pow(10, 9)
const BASE_REWARD_CLAMP_THRESHOLD: u64 = 100000000; // pow(10, 8)
/// Attempts at making the miner transaction's reward agree with its own weight.
const MINER_TX_ATTEMPTS: usize = 10;

/// This function makes a new block for a miner to mine the hash for
///
/// `from_block` is the block to build on, the most recent one if [`None`].
///
/// ref <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_core/blockchain.cpp#L1547-L1812>
#[instrument(level = "debug", skip_all, name = "create_block_template")]
pub(crate) async fn create_block_template(
    blockchain_read: &mut BlockchainReadHandle,
    blockchain_context: &BlockchainContextService,
    txpool_read: &mut TxpoolReadHandle,
    miner_address: &MoneroAddress,
    from_block: Option<BlockHash>,
    extra_nonce: &[u8],
    include_sensitive_txs: bool,
) -> Result<BlockTemplate, Error> {
    // later TODO: add separate cache for block template creation?
    let context = blockchain_context
        .blockchain_context_snapshot()
        .context_to_verify_block
        .clone();

    // later TODO: build on other blocks and alt-chains, as monerod does.
    if from_block.is_some_and(|from_block| from_block != context.top_hash) {
        return Err(anyhow!(
            "Creating a block template from a previous block other than the most recent is not supported"
        ));
    }

    let BlockTemplateFill {
        transaction_hashes,
        total_weight,
        fee,
    } = fill_block_template(
        &context,
        blockchain_read,
        txpool_read,
        include_sensitive_txs,
    )
    .await?;

    let miner_tx = create_miner_tx(&context, miner_address, extra_nonce, total_weight, fee)?;
    // Unlike monerod's estimate, this is exactly what the miner transaction pays.
    let expected_reward = miner_tx
        .prefix()
        .outputs
        .iter()
        .filter_map(|output| output.amount)
        .sum();

    let timestamp = current_unix_timestamp();
    let timestamp = if let Some(median) = context.median_block_timestamp {
        if timestamp < median {
            median
        } else {
            timestamp
        }
    } else {
        timestamp
    };

    let block = Block::new(
        BlockHeader {
            hardfork_version: context.current_hf.as_u8(),
            hardfork_signal: HardFork::LATEST.as_u8(),
            timestamp,
            previous: context.top_hash,
            nonce: 0,
        },
        miner_tx,
        transaction_hashes,
    )
    .ok_or_else(|| anyhow!("Failed to create block template"))?;
    let reserved_offset = reserved_offset(&block, extra_nonce.len())?;

    // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1870-L1875>
    let height = context.chain_height;
    let seed_height = randomx_seed_height(height);
    let next_seed_height = randomx_seed_height(height + RX_SEEDHASH_EPOCH_LAG);

    // Zeroed before RandomX, and for a next seed that is the current one.
    let mut seed_hash = [0; 32];
    let mut next_seed_hash = [0; 32];
    if context.current_hf >= HardFork::V12 {
        seed_hash = get_block_hash_main_chain(blockchain_read, seed_height).await?;
        if next_seed_height != seed_height {
            next_seed_hash = get_block_hash_main_chain(blockchain_read, next_seed_height).await?;
        }
    }

    Ok(BlockTemplate {
        block,
        reserved_offset,
        difficulty: context.next_difficulty,
        height: usize_to_u64(height),
        expected_reward,
        seed_height: usize_to_u64(seed_height),
        seed_hash,
        next_seed_hash,
    })
}

/// Finds a nonce at or after `starting_nonce` that satisfies `difficulty`.
///
/// Like monerod's, the search cannot be cancelled. It keeps
/// a `rayon` thread busy until it ends, even if the caller is gone.
///
/// ref <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_basic/miner.cpp#L476-L491>
pub(crate) async fn find_nonce(
    blockchain_context: &mut BlockchainContextService,
    mut block: Block,
    height: usize,
    difficulty: u128,
    starting_nonce: u32,
) -> Result<Block, Error> {
    block.header.nonce = starting_nonce;
    if difficulty <= 1 {
        return Ok(block);
    }

    let hf = HardFork::from_version(block.header.hardfork_version)?;
    let rx_vm = if hf >= HardFork::V12 {
        let BlockChainContextResponse::RxVms(mut rx_vms) = blockchain_context
            .ready()
            .await
            .map_err(|e| anyhow!(e))?
            .call(BlockChainContextRequest::CurrentRxVms)
            .await
            .map_err(|e| anyhow!(e))?
        else {
            unreachable!();
        };

        // `calculate_pow_hash` panics without a VM.
        Some(
            rx_vms
                .remove(&randomx_seed_height(height))
                .ok_or_else(|| anyhow!("No RandomX VM for height {height}"))?,
        )
    } else {
        None
    };

    rayon_spawn_async(move || loop {
        let pow_hash =
            calculate_pow_hash(rx_vm.as_deref(), &block.serialize_pow_hash(), height, &hf)?;
        if check_block_pow(&pow_hash, difficulty).is_ok() {
            return Ok(block);
        }

        block.header.nonce = block.header.nonce.wrapping_add(1);
        if block.header.nonce == starting_nonce {
            return Err(anyhow!("No nonce satisfies difficulty {difficulty}"));
        }
    })
    .await
}

/// return type of [`fill_block_template`]
struct BlockTemplateFill {
    transaction_hashes: Vec<TransactionHash>,
    total_weight: usize,
    fee: u64,
}

/// accumulated state of the block being filled with txs
#[derive(Debug)]
struct BlockTemplateBuilder {
    total_weight: usize,
    fee: u64,
    best_coinbase: u64,
}

impl BlockTemplateBuilder {
    fn new(context: &ContextToVerifyBlock) -> Self {
        Self {
            total_weight: 0,
            fee: 0,
            best_coinbase: calculate_block_reward(
                0,
                context.median_weight_for_block_reward,
                context.already_generated_coins,
                context.current_hf,
            ),
        }
    }
}

/// reads txs in pool and selects the ones to include in the next block
///
/// returns the selected transaction hashes and the totals needed to construct the miner tx ([`BlockTemplateFill`])
///
/// ref <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_core/tx_pool.cpp#L1617-L1760>
async fn fill_block_template(
    context: &ContextToVerifyBlock,
    blockchain_read: &mut BlockchainReadHandle,
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
) -> Result<BlockTemplateFill, Error> {
    let tx_entries = backlog(txpool_read).await?;

    tracing::debug!(
        "Filling block template, median weight {}, {} txes in the pool",
        context.median_weight_for_block_reward,
        tx_entries.len()
    );
    let selected = select_transactions(context, tx_entries, include_sensitive_txs);

    // The pool may have changed since the backlog was read, and it learns about new
    // blocks asynchronously, so leave out transactions that are gone or already spent.
    //
    // monerod verifies the inputs of every transaction it considers again. Here that
    // would be the ring signatures of the whole pool on every call, without a cache.
    let (mut txs, _) = txs_for_block(txpool_read, selected.clone()).await?;
    let key_images: Vec<[u8; 32]> = txs.values().flat_map(|tx| tx_key_images(&tx.tx)).collect();
    let mut seen_key_images = key_images_spent(blockchain_read, key_images).await?;

    let mut fill = BlockTemplateFill {
        transaction_hashes: Vec::with_capacity(selected.len()),
        total_weight: 0,
        fee: 0,
    };
    for tx_hash in selected {
        let Some(tx) = txs.remove(&tx_hash) else {
            continue;
        };

        let key_images = tx_key_images(&tx.tx);
        if key_images.iter().any(|ki| seen_key_images.contains(ki)) {
            tracing::debug!("key images already seen");
            continue;
        }
        seen_key_images.extend(key_images);

        fill.transaction_hashes.push(tx_hash);
        fill.total_weight += tx.tx_weight;
        fill.fee = fill.fee.saturating_add(tx.fee);
    }

    Ok(fill)
}

/// selects the transactions of the next block from the pool's `tx_entries`, by weight and fee
///
/// Like monerod, this considers them by fee per weight, then by age,
/// and leaves out stem transactions unless `include_sensitive_txs` is set.
fn select_transactions(
    context: &ContextToVerifyBlock,
    mut tx_entries: Vec<TxEntry>,
    include_sensitive_txs: bool,
) -> Vec<TransactionHash> {
    let median_weight = context.median_weight_for_block_reward;
    let max_total_weight = if context.current_hf >= HardFork::V5 {
        2 * median_weight
    } else {
        (130 * median_weight) / 100
    } - CRYPTONOTE_COINBASE_BLOB_RESERVED_SIZE;

    if !include_sensitive_txs {
        tx_entries.retain(|tx| !tx.private);
    }
    tx_entries.sort_unstable_by(|a, b| {
        // `fee / weight` compared by cross-multiplication.
        let a_rate = u128::from(a.fee) * u128::from(usize_to_u64(b.weight));
        let b_rate = u128::from(b.fee) * u128::from(usize_to_u64(a.weight));
        b_rate
            .cmp(&a_rate)
            .then_with(|| a.received_at.cmp(&b.received_at))
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut builder = BlockTemplateBuilder::new(context);
    tx_entries
        .iter()
        .filter_map(|entry| consider_transaction(context, max_total_weight, entry, &mut builder))
        .collect()
}

/// returns the transaction's hash if it should be added, `None` if it should be skipped
/// The mutable totals are only updated if the transaction is added.
///
/// transactions are skipped if they
/// - don't fit in the block weight limits, or
/// - would decrease the coinbase reward
#[instrument(level = "trace", ret, skip_all, fields(tx.weight = entry.weight, total_weight = builder.total_weight))]
fn consider_transaction(
    context: &ContextToVerifyBlock,
    max_total_weight: usize,
    entry: &TxEntry,
    builder: &mut BlockTemplateBuilder,
) -> Option<TransactionHash> {
    // later TODO: check if pruned, continue if it is

    if builder.total_weight + entry.weight > max_total_weight {
        tracing::trace!("exceeds maximum block weight");
        return None;
    }

    let coinbase = if context.current_hf >= HardFork::V5 {
        let block_reward = calculate_block_reward(
            builder.total_weight + entry.weight,
            context.median_weight_for_block_reward,
            context.already_generated_coins,
            context.current_hf,
        );
        let coinbase = block_reward
            .saturating_add(builder.fee)
            .saturating_add(entry.fee);
        if coinbase < builder.best_coinbase * ACCEPT_THRESHOLD {
            tracing::trace!("would decrease coinbase");
            return None;
        }
        Some(coinbase)
    } else if builder.total_weight > context.median_weight_for_block_reward {
        tracing::trace!("exceeds median block weight");
        return None;
    } else {
        // TODO: coinbase isn't updated if hf < V5. this is also a minor bug in monerod as far as I've seen it
        None
    };

    builder.total_weight += entry.weight;
    builder.fee = builder.fee.saturating_add(entry.fee);
    if let Some(coinbase) = coinbase {
        builder.best_coinbase = coinbase;
    }

    Some(entry.id)
}

/// The key images `tx` spends.
fn tx_key_images(tx: &Transaction) -> Vec<[u8; 32]> {
    tx.prefix()
        .inputs
        .iter()
        .filter_map(|input| match input {
            Input::ToKey { key_image, .. } => Some(key_image.to_bytes()),
            Input::Gen(_) => None,
        })
        .collect()
}

/// The `key_images` that the main chain has spent.
async fn key_images_spent(
    blockchain_read: &mut BlockchainReadHandle,
    key_images: Vec<[u8; 32]>,
) -> Result<HashSet<[u8; 32]>, Error> {
    if key_images.is_empty() {
        return Ok(HashSet::new());
    }

    let BlockchainResponse::KeyImagesSpentVec(spent) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::KeyImagesSpentVec(key_images.clone()))
        .await?
    else {
        unreachable!();
    };

    Ok(key_images
        .into_iter()
        .zip(spent)
        .filter_map(|(key_image, spent)| spent.then_some(key_image))
        .collect())
}

async fn get_block_hash_main_chain(
    blockchain_read: &mut BlockchainReadHandle,
    height: usize,
) -> Result<BlockHash, Error> {
    let BlockchainResponse::BlockHash(hash) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockHash(
            height,
            cuprate_types::Chain::Main,
        ))
        .await?
    else {
        unreachable!();
    };

    Ok(hash)
}

/// Offset of the extra nonce within the serialized `block`, zero without one.
///
/// monerod finds it by searching the blob for the transaction public key:
/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1877-L1897>
fn reserved_offset(block: &Block, extra_nonce_len: usize) -> Result<u64, Error> {
    if extra_nonce_len == 0 {
        return Ok(0);
    }

    let miner_tx = block.miner_transaction();
    // `extra` ends the prefix, which a v2 miner transaction follows with one `RctType::Null` byte.
    let after_extra = usize::from(matches!(miner_tx, Transaction::V2 { .. }));
    let extra_end = block.header.serialize().len() + miner_tx.serialize().len() - after_extra;

    // The nonce follows the public key and its tag, then its own tag and length.
    extra_end
        .checked_sub(miner_tx.prefix().extra.len())
        .map(|extra_start| usize_to_u64(extra_start + 1 + 32 + 2))
        .ok_or_else(|| anyhow!("Failed to calculate offset for reserved bytes"))
}

/// Constructs the miner transaction of a block whose other transactions weigh `total_weight` and pay `fee`.
///
/// ref <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_core/blockchain.cpp#L1748-L1810>
fn create_miner_tx(
    context: &ContextToVerifyBlock,
    miner_address: &MoneroAddress,
    extra_nonce: &[u8],
    total_weight: usize,
    fee: u64,
) -> Result<Transaction, Error> {
    //  two-phase miner transaction generation: we don't know exact block weight until we prepare block, but we don't know reward until we know
    //  block weight, so first miner transaction generated with fake amount of money, and with phase we know think we know expected block weight
    let miner_tx = construct_miner_tx(context, miner_address, extra_nonce, total_weight, fee)?;
    let mut cumulative_weight = total_weight + miner_tx.weight();

    for _ in 0..MINER_TX_ATTEMPTS {
        let mut miner_tx =
            construct_miner_tx(context, miner_address, extra_nonce, cumulative_weight, fee)?;
        let coinbase_weight = miner_tx.weight();

        if coinbase_weight > cumulative_weight - total_weight {
            tracing::debug!("Miner tx weight is greater than before.");
            cumulative_weight = total_weight + coinbase_weight;
            continue;
        }

        if coinbase_weight < cumulative_weight - total_weight {
            let delta = cumulative_weight - total_weight - coinbase_weight;
            tracing::debug!("Miner tx weight is less than before, adding {delta} zero-bytes.");
            miner_tx.prefix_mut().extra.put_bytes(0, delta);

            // The length of `extra` is a varint, which the padding may have grown by a byte.
            if cumulative_weight != total_weight + miner_tx.weight() {
                ensure!(
                    cumulative_weight + 1 == total_weight + miner_tx.weight(),
                    "Failed to pad the miner tx"
                );
                miner_tx.prefix_mut().extra.pop();

                if cumulative_weight != total_weight + miner_tx.weight() {
                    // One byte less shrank the varint again, so aim for a heavier block.
                    // monerod adds `delta - 1`, which gets it nowhere when `delta` is one.
                    cumulative_weight += (delta - 1).max(1);
                    continue;
                }
            }
        }

        return Ok(miner_tx);
    }

    Err(anyhow!("Couldn't construct a miner_tx."))
}

/// constructs miner tx for a block
/// the miner tx pays the block reward (base emission plus the collected `fee`) to `miner_address`
///
/// Note: use [`create_miner_tx`] if you don't know the final weight yet, as the reward depends on `block_weight`.
///
/// ref <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_core/cryptonote_tx_utils.cpp#L78-L183>
fn construct_miner_tx(
    context: &ContextToVerifyBlock,
    miner_address: &MoneroAddress,
    extra_nonce: &[u8],
    block_weight: usize,
    fee: u64,
) -> Result<Transaction, Error> {
    let tx_key = generate_key_pair();

    // The reward is only defined for weights that pass this check.
    check_block_weight(block_weight, context.median_weight_for_block_reward)?;
    let base_reward = calculate_block_reward(
        block_weight,
        context.median_weight_for_block_reward,
        context.already_generated_coins,
        context.current_hf,
    );
    let mut block_reward = base_reward
        .checked_add(fee)
        .ok_or_else(|| anyhow!("Block reward overflow"))?;

    // Hard forks 2 and 3 cut the low digits, which makes for fewer outputs.
    // The miner may claim less than the reward there, and the rest stays in the emission.
    // Unlike monerod, this never cuts into the fees, which no block may leave unclaimed.
    let cut = block_reward % BASE_REWARD_CLAMP_THRESHOLD;
    if matches!(context.current_hf, HardFork::V2 | HardFork::V3) && cut <= base_reward {
        block_reward -= cut;
    }

    let mut out_amounts = Vec::new();
    decompose_amount_into_digits(
        block_reward,
        if context.current_hf >= HardFork::V2 {
            0
        } else {
            DEFAULT_DUST_THRESHOLD
        },
        &mut out_amounts,
    );

    let max_outs = if context.current_hf >= HardFork::V4 {
        1
    } else {
        11
    };

    // the genesis block was not decomposed, for unknown reasons
    if context.chain_height == 0 || context.current_hf >= HardFork::V4 {
        while max_outs < out_amounts.len() {
            out_amounts[1] += out_amounts[0];
            for n in 1..out_amounts.len() {
                out_amounts[n - 1] = out_amounts[n];
            }
            out_amounts.pop();
        }
    } else {
        ensure!(out_amounts.len() <= max_outs, "max_out exceeded");
    }

    let derivation = generate_key_derivation(miner_address.view(), tx_key.private);
    let outputs = out_amounts
        .into_iter()
        .enumerate()
        .map(|(i, amount)| {
            let out_eph_public_key = derive_public_key(derivation, i, miner_address.spend())?;

            let view_tag = if context.current_hf >= HardFork::V15 {
                Some(derive_view_tag(derivation, i)?)
            } else {
                None
            };
            Ok(Output {
                amount: Some(amount),
                key: out_eph_public_key.compress(),
                view_tag,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let prefix = TransactionPrefix {
        additional_timelock: Timelock::Block(context.chain_height + MINER_TX_TIME_LOCKED_BLOCKS),
        inputs: vec![Input::Gen(context.chain_height)],
        outputs,
        extra: get_extra_field(tx_key.public, extra_nonce)?,
    };

    Ok(if context.current_hf >= HardFork::V4 {
        Transaction::V2 {
            prefix,
            proofs: None,
        }
    } else {
        Transaction::V1 {
            prefix,
            signatures: Vec::new(),
        }
    })
}

struct TransactionKey {
    public: CompressedPoint,
    private: Scalar,
}

/// get's a random scalar -> private key (scalar)
/// private key * G -> public key (point)
fn generate_key_pair() -> TransactionKey {
    let mut rng = rand::thread_rng();
    let private = loop {
        let key = curve25519_dalek::Scalar::random(&mut rng);
        if key != curve25519_dalek::Scalar::ZERO {
            break key;
        }
    };
    let public = (curve25519_dalek::constants::ED25519_BASEPOINT_POINT * private).compress();

    TransactionKey {
        public: CompressedPoint::from(public.0),
        private: Scalar::from(private),
    }
}

/// serialize public key and extra nonce to bytes
///
/// ref <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_core/cryptonote_tx_utils.cpp#L82-L89>
fn get_extra_field(public_key: CompressedPoint, extra_nonce: &[u8]) -> Result<Vec<u8>, Error> {
    let mut extra = Vec::with_capacity(33 + 2 + extra_nonce.len());

    extra.put_u8(TX_EXTRA_TAG_PUBKEY);
    extra.extend_from_slice(&public_key.to_bytes());

    if !extra_nonce.is_empty() {
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_basic/cryptonote_format_utils.cpp#L726-L740>
        extra.put_u8(TX_EXTRA_NONCE);
        extra.put_u8(
            u8::try_from(extra_nonce.len()).map_err(|_| anyhow!("Too big extra_nonce size"))?,
        );
        extra.extend_from_slice(extra_nonce);
    }
    // sorting doesn't have to be done here as there are just these 2 fields (which are in order)

    Ok(extra)
}

/// Decomposes `amount` into canonical "digit" chunks (e.g. `62387455827` ->
/// `455827 + 7000000 + 80000000 + 300000000 + 2000000000 + 60000000000`,
/// where `455827 <= dust_threshold`), pushing every dust and non-dust chunk to `out_amounts`
///
/// ref <https://github.com/monero-project/monero/blob/24272e4a3b2e812cc2a569d95b177ecef4b91e46/src/cryptonote_basic/cryptonote_format_utils.h#L239-L280>
fn decompose_amount_into_digits(mut amount: u64, dust_threshold: u64, out_amounts: &mut Vec<u64>) {
    if amount == 0 {
        return;
    }

    let mut is_dust_handled = false;
    let mut dust: u64 = 0;
    let mut order: u64 = 1;

    while amount != 0 {
        let chunk = (amount % 10) * order;
        amount /= 10;
        // Saturates past the last digit of the largest amounts, where it is not used again.
        order = order.saturating_mul(10);

        if dust + chunk <= dust_threshold {
            dust += chunk;
        } else {
            if !is_dust_handled && dust != 0 {
                out_amounts.push(dust);
                is_dust_handled = true;
            }
            if chunk != 0 {
                out_amounts.push(chunk);
            }
        }
    }

    if !is_dust_handled && dust != 0 {
        out_amounts.push(dust);
    }
}

/// ref <https://github.com/monero-project/monero/blob/24272e4a3b2e812cc2a569d95b177ecef4b91e46/src/crypto/crypto.cpp#L254-L267>
fn generate_key_derivation(view: Point, private: Scalar) -> Point {
    Point::from((view.into() * private.into()).mul_by_cofactor())
}

/// ref <https://github.com/monero-project/monero/blob/24272e4a3b2e812cc2a569d95b177ecef4b91e46/src/crypto/crypto.cpp#L281-L299>
fn derive_public_key(derivation: Point, i: usize, spend: Point) -> Result<Point, Error> {
    let mut buf = Vec::from(derivation.compress().to_bytes());
    VarInt::write(&i, &mut buf)?;
    let scalar = Scalar::hash(buf).into();
    Ok(Point::from(
        curve25519_dalek::constants::ED25519_BASEPOINT_POINT * scalar + spend.into(),
    ))
}

/// ref <https://github.com/monero-project/monero/blob/24272e4a3b2e812cc2a569d95b177ecef4b91e46/src/crypto/crypto.cpp#L867-L890>
fn derive_view_tag(derivation: Point, i: usize) -> Result<u8, Error> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"view_tag");
    buf.extend_from_slice(&derivation.compress().to_bytes());
    VarInt::write(&i, &mut buf)?;
    Ok(keccak256(buf)[0])
}

#[cfg(test)]
mod tests {
    use curve25519_dalek::EdwardsPoint;
    use monero_address::{AddressType, Network};

    use cuprate_consensus_rules::{is_decomposed_amount, miner_tx::check_miner_tx};

    use super::*;

    const MEDIAN_WEIGHT: usize = 300_000;

    fn context(hf: HardFork) -> ContextToVerifyBlock {
        ContextToVerifyBlock {
            median_weight_for_block_reward: MEDIAN_WEIGHT,
            effective_median_weight: MEDIAN_WEIGHT,
            top_hash: [0; 32],
            median_block_timestamp: None,
            chain_height: 1_000,
            current_hf: hf,
            next_difficulty: 1,
            already_generated_coins: 0,
        }
    }

    /// A miner's address with its secret spend and view keys.
    fn miner() -> (MoneroAddress, [curve25519_dalek::Scalar; 2]) {
        let secrets = [2_u8, 3].map(|seed| Scalar::hash([seed]).into());
        let [spend, view] = secrets.map(|secret| Point::from(EdwardsPoint::mul_base(&secret)));
        (
            MoneroAddress::new(Network::Mainnet, AddressType::Legacy, spend, view),
            secrets,
        )
    }

    fn entry(id: u8, weight: usize, fee: u64, received_at: u64) -> TxEntry {
        TxEntry {
            id: [id; 32],
            weight,
            fee,
            private: false,
            received_at,
        }
    }

    /// The first byte of each transaction selected from `tx_entries`.
    fn selected(hf: HardFork, tx_entries: Vec<TxEntry>, include_sensitive_txs: bool) -> Vec<u8> {
        select_transactions(&context(hf), tx_entries, include_sensitive_txs)
            .into_iter()
            .map(|tx_hash| tx_hash[0])
            .collect()
    }

    #[test]
    fn stem_txs_are_only_selected_when_asked() {
        let mut stem = entry(2, 1, 1, 0);
        stem.private = true;
        let tx_entries = vec![entry(1, 1, 1, 0), stem];

        assert_eq!(selected(HardFork::V16, tx_entries.clone(), false), [1]);
        assert_eq!(selected(HardFork::V16, tx_entries, true), [1, 2]);
    }

    #[test]
    fn txs_are_considered_by_fee_rate_then_age_then_id() {
        let tx_entries = vec![
            entry(5, 2_000, 2_000, 9),
            entry(4, 1_000, 1_000, 9),
            entry(3, 1_000, 1_000, 1),
            entry(2, 3_000, 9_000, 9),
            entry(1, 1_000, 2_999, 0),
        ];

        assert_eq!(selected(HardFork::V16, tx_entries, false), [2, 1, 3, 4, 5]);
    }

    #[test]
    fn selection_leaves_room_for_the_miner_tx() {
        // Fees that outweigh any penalty.
        let fee = 1_000_000_000_000_000;
        let heavy = |id| entry(id, 100_000, fee, 0);
        let light = |id, weight| entry(id, weight, fee / 4, 0);

        // From hard fork 5, up to twice the median weight.
        let mut tx_entries: Vec<_> = (1..=6).map(heavy).collect();
        tx_entries.push(light(7, 100_000 - CRYPTONOTE_COINBASE_BLOB_RESERVED_SIZE));
        tx_entries.push(entry(8, 1, 1_000_000_000, 0));
        assert_eq!(
            selected(HardFork::V16, tx_entries, false),
            [1, 2, 3, 4, 5, 7]
        );

        // Before, up to 130% of it, and nothing more once past the median.
        let tx_entries = vec![heavy(1), heavy(2), heavy(3), light(4, 90_000)];
        assert_eq!(selected(HardFork::V4, tx_entries, false), [1, 2, 3]);
        let tx_entries = vec![
            heavy(1),
            heavy(2),
            heavy(3),
            light(4, 50_000),
            entry(5, 1, 1, 0),
        ];
        assert_eq!(selected(HardFork::V4, tx_entries, false), [1, 2, 3, 4]);
    }

    #[test]
    fn selection_never_lowers_the_coinbase() {
        let tx_entries = vec![
            // Fills the block up to the median weight.
            entry(1, MEDIAN_WEIGHT, 1_000_000_000_000_000, 0),
            // Each costs about 1% of the 35 XMR reward.
            entry(2, 30_000, 3, 0),
            entry(3, 30_000, 2, 0),
            entry(4, 30_000, 0, 0),
        ];
        assert_eq!(selected(HardFork::V16, tx_entries.clone(), false), [1]);

        // One that pays for its penalty is worth it.
        let mut paying = tx_entries;
        paying[2].fee = 1_000_000_000_000;
        assert_eq!(selected(HardFork::V16, paying, false), [1, 3]);
    }

    #[test]
    fn amounts_decompose_into_digits_and_dust() {
        let decompose = |amount, dust_threshold| {
            let mut out_amounts = Vec::new();
            decompose_amount_into_digits(amount, dust_threshold, &mut out_amounts);
            out_amounts
        };

        assert_eq!(decompose(0, 0), [0_u64; 0]);
        assert_eq!(decompose(1_203, 0), [3, 200, 1_000]);
        assert_eq!(
            decompose(62_387_455_827, DEFAULT_DUST_THRESHOLD),
            [387_455_827, 2_000_000_000, 60_000_000_000]
        );

        let digits = decompose(u64::MAX, 0);
        assert!(digits.iter().all(is_decomposed_amount));
        assert_eq!(digits.iter().sum::<u64>(), u64::MAX);
    }

    #[test]
    fn miner_txs_follow_consensus_rules() {
        let (address, [spend_secret, view_secret]) = miner();
        let spend = EdwardsPoint::mul_base(&spend_secret);
        let fee = 12_345;

        for hf in (1..=HardFork::LATEST.as_u8()).map(|v| HardFork::from_version(v).unwrap()) {
            let context = context(hf);

            // Below and above the median weight, where the reward is penalized.
            for (total_weight, extra_nonce) in [
                (0, [7; 8].as_slice()),
                (400_000, &[]),
                (400_000, &[7; TX_EXTRA_NONCE_MAX_COUNT]),
            ] {
                let tx =
                    create_miner_tx(&context, &address, extra_nonce, total_weight, fee).unwrap();
                let block_weight = tx.weight() + total_weight;
                let outputs = &tx.prefix().outputs;
                let paid = outputs.iter().map(|o| o.amount.unwrap()).sum::<u64>();

                let reward = check_miner_tx(
                    &tx,
                    fee,
                    context.chain_height,
                    block_weight,
                    MEDIAN_WEIGHT,
                    context.already_generated_coins,
                    hf,
                )
                .unwrap();
                assert_eq!(reward + fee, paid);

                // Only hard forks 2 and 3 claim less than the full reward, and barely.
                let full = calculate_block_reward(block_weight, MEDIAN_WEIGHT, 0, hf) + fee;
                let clamped = matches!(hf, HardFork::V2 | HardFork::V3);
                assert_eq!(
                    full - paid,
                    if clamped {
                        full % BASE_REWARD_CLAMP_THRESHOLD
                    } else {
                        0
                    }
                );
                assert_eq!(outputs.len() == 1, hf >= HardFork::V4);

                // The miner's wallet finds every output with its view key.
                let extra = &tx.prefix().extra;
                assert_eq!(extra[0], TX_EXTRA_TAG_PUBKEY);
                let tx_public = CompressedPoint::from(<[u8; 32]>::try_from(&extra[1..33]).unwrap())
                    .decompress()
                    .unwrap();
                let derivation = (view_secret * tx_public.into())
                    .mul_by_cofactor()
                    .compress()
                    .to_bytes();
                for (index, output) in outputs.iter().enumerate() {
                    let mut derivation = derivation.to_vec();
                    VarInt::write(&index, &mut derivation).unwrap();

                    let key = output.key.decompress().unwrap().into();
                    assert_eq!(
                        key - EdwardsPoint::mul_base(&Scalar::hash(&derivation).into()),
                        spend
                    );
                    assert_eq!(
                        output.view_tag,
                        (hf >= HardFork::V15)
                            .then(|| keccak256([b"view_tag".as_slice(), &derivation].concat())[0])
                    );
                }
            }
        }
    }

    #[test]
    fn blocks_too_heavy_for_a_reward_are_errors() {
        let (address, _) = miner();
        let context = context(HardFork::V16);

        assert!(create_miner_tx(&context, &address, &[], 2 * MEDIAN_WEIGHT - 50, 0).is_err());
        assert!(create_miner_tx(&context, &address, &[], 2 * MEDIAN_WEIGHT + 1, 0).is_err());
    }

    #[test]
    fn reserved_offset_points_at_extra_nonce() {
        let (address, _) = miner();
        let nonce = [0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe];

        for hf in [HardFork::V1, HardFork::V16] {
            let mut miner_tx = construct_miner_tx(&context(hf), &address, &nonce, 0, 0).unwrap();
            // Padding, as `create_miner_tx` may add, follows the nonce.
            miner_tx.prefix_mut().extra.put_bytes(0, 100);

            let block = Block::new(
                BlockHeader {
                    hardfork_version: hf.as_u8(),
                    hardfork_signal: hf.as_u8(),
                    timestamp: 1_700_000_000,
                    previous: [0x11; 32],
                    nonce: 0,
                },
                miner_tx,
                vec![[0x22; 32]],
            )
            .unwrap();

            let offset = usize::try_from(reserved_offset(&block, nonce.len()).unwrap()).unwrap();
            assert_eq!(block.serialize()[offset..offset + nonce.len()], nonce);
            assert_eq!(reserved_offset(&block, 0).unwrap(), 0);
        }
    }
}
