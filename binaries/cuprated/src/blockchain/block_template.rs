//! Handles creating a block-template with all the relevant methods.
//!
//! This mainly includes:
//! - (finding the parent block: not done as `from_block` is not used by monero-oxide; using the most recent (genesis for `FakeChain`))
//! - (selecting txs from the pool: not used in monero-oxide)
//! - constructing a miner transaction
use anyhow::Error;
use bytes::BufMut;
use cuprate_consensus::{
    batch_verifier::MultiThreadedBatchVerifier,
    transactions::contextual_data::batch_get_ring_member_info, BlockchainContext,
    BlockchainContextService, BlockchainReadRequest, BlockchainResponse, VerificationContext,
};
use cuprate_consensus_rules::{
    blocks::RX_SEEDHASH_EPOCH_LAG,
    miner_tx::{calculate_block_reward, MINER_TX_TIME_LOCKED_BLOCKS},
    transactions::{
        check_transaction_contextual, check_transaction_semantic, TxRingMembersInfo,
        CRYPTONOTE_COINBASE_BLOB_RESERVED_SIZE,
    },
};
use cuprate_helper::{cast::usize_to_u64, time::current_unix_timestamp};
use cuprate_hex::HexVec;
use monero_address::MoneroAddress;

use cuprate_blockchain::{service::BlockchainReadHandle, types::BlockHash};
use cuprate_txpool::{service::TxpoolReadHandle, types::TransactionHash, TxEntry};
use cuprate_types::{BlockTemplate, HardFork, TransactionVerificationData};
use monero_oxide::{
    block::{Block, BlockHeader},
    ed25519::{CompressedPoint, Point, Scalar},
    io::VarInt,
    primitives::keccak256,
    transaction::{Input, Output, Timelock, Transaction, TransactionPrefix},
};
use tower::{BoxError, Service, ServiceExt};
use tracing::instrument;

use crate::{
    blockchain::ConsensusBlockchainReadHandle,
    txpool::read::{backlog, txs_for_block},
};

const ACCEPT_THRESHOLD: u64 = 1;
const TX_EXTRA_NONCE_MAX_COUNT: usize = 255;
const TX_EXTRA_TAG_PUBKEY: u8 = 0x01;
const _TX_EXTRA_NONCE: u8 = 0x02;
const DEFAULT_DUST_THRESHOLD: u64 = 2000000000; // 2 * pow(10, 9)

/// This function makes a new block for a miner to mine the hash for
///
/// ref <https://github.com/monero-project/monero/blob/1c079ffa9117e7429d16ebc6d627ecdb0a952bd4/src/cryptonote_core/blockchain.cpp#L1521-L1790>
#[instrument(level = "debug", skip_all, name = "create_block_template")]
pub(crate) async fn create_block_template(
    blockchain_read: &mut BlockchainReadHandle,
    blockchain_context: &BlockchainContextService,
    txpool_read: &mut TxpoolReadHandle,
    miner_address: &MoneroAddress,
    from_block: BlockHash,
    extra_nonce: HexVec,
    include_sensitive_txs: bool,
) -> Result<BlockTemplate, Error> {
    // later TODO: add separate cache for block template creation?
    let context = blockchain_context.blockchain_context_snapshot();

    assert_eq!(from_block, BlockHash::default(), "creating a block template from a previous block other than the most recent (or alt-chain) is currently not supported.");

    let BlockTemplateFill {
        transaction_hashes,
        total_weight,
        fee,
        best_coinbase,
    } = fill_block_template(
        &context,
        blockchain_read,
        txpool_read,
        include_sensitive_txs,
    )
    .await?;

    let miner_tx = create_miner_tx(&context, miner_address, &extra_nonce, total_weight, fee)?;

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
            hardfork_version: HardFork::LATEST as u8,
            hardfork_signal: HardFork::LATEST as u8,
            timestamp,
            previous: context.top_hash,
            nonce: 0,
        },
        miner_tx,
        transaction_hashes,
    )
    .unwrap();

    let height = context.chain_height; // later TODO: the height may be provided by `from_block`
    let seed_height = cuprate_consensus_rules::blocks::randomx_seed_height(height); // later TODO: this is not correct for alt-chains,
    let seed_hash = get_block_hash_main_chain(blockchain_read, seed_height).await?; // later TODO: this is not correct for alt-chains,

    let next_seed_hash = get_block_hash_main_chain(
        blockchain_read,
        cuprate_consensus_rules::blocks::randomx_seed_height(seed_height + RX_SEEDHASH_EPOCH_LAG),
    )
    .await?;

    Ok(BlockTemplate {
        block,
        reserved_offset: 0,
        difficulty: context.next_difficulty, // later TODO: this is not correct for alt-chains,
        height: usize_to_u64(height),
        expected_reward: best_coinbase,
        seed_height: usize_to_u64(seed_height),
        seed_hash,
        next_seed_hash,
    })
}

/// return type of [`fill_block_template`]
struct BlockTemplateFill {
    transaction_hashes: Vec<TransactionHash>,
    total_weight: usize,
    fee: u64,
    best_coinbase: u64,
}

/// accumulated state of the block being filled with txs
#[derive(Debug)]
struct BlockTemplateBuilder {
    total_weight: usize,
    fee: u64,
    best_coinbase: u64,
}

impl BlockTemplateBuilder {
    fn new(context: &BlockchainContext) -> Self {
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
/// ref <https://github.com/monero-project/monero/blob/e0f36de6149ebfbbe092e783fffe0430d97f3e1f/src/cryptonote_core/tx_pool.cpp#L1595-L1739>
async fn fill_block_template(
    context: &BlockchainContext,
    blockchain_read: &BlockchainReadHandle,
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
) -> Result<BlockTemplateFill, Error> {
    let max_total_weight =
        2 * context.median_weight_for_block_reward - CRYPTONOTE_COINBASE_BLOB_RESERVED_SIZE;
    // later TODO: handle pre V5

    let mut tx_entries = backlog(txpool_read).await?;
    if !include_sensitive_txs {
        tx_entries.retain(|tx| !tx.private);
    }

    let tx_hashes = tx_entries.iter().map(|tx| tx.id).collect();
    let (verification_data, _) = txs_for_block(txpool_read, tx_hashes).await?;

    let candidates: Vec<_> = tx_entries
        .iter()
        .filter_map(|entry| verification_data.get(&entry.id).map(|tx| (entry, tx)))
        .collect();

    let verification_txs: Vec<_> = candidates.iter().map(|(_, tx)| *tx).collect();
    let consensus_read =
        ConsensusBlockchainReadHandle::new(blockchain_read.clone(), BoxError::from);
    let mut verification_context = VerificationContext::Database(consensus_read);

    let ring_members = batch_get_ring_member_info(
        verification_txs.iter().copied(),
        context.current_hf,
        &mut verification_context,
    )
    .await?;

    let mut builder = BlockTemplateBuilder::new(context);

    tracing::info!(
        "Filling block template, median weight {}, {} txes in the pool",
        context.median_weight_for_block_reward,
        tx_entries.len()
    );
    let transaction_hashes = candidates
        .iter()
        .zip(ring_members.iter())
        .filter_map(|((entry, tx), tx_ring_members_info)| {
            consider_transaction(
                context,
                max_total_weight,
                entry,
                tx,
                tx_ring_members_info,
                &mut builder,
            )
        })
        .collect();

    Ok(BlockTemplateFill {
        transaction_hashes,
        total_weight: builder.total_weight,
        fee: builder.fee,
        best_coinbase: builder.best_coinbase,
    })
}

/// returns the transaction's hash if it should be added, `None` if it should be skipped
/// The mutable totals are only updated if the transaction is added.
///
/// transactions are skipped if they
/// - don't fit in the block weight limits
/// - would decrease the coinbase reward, or
/// - fail the contextual/semantic consensus checks
#[tracing::instrument(ret, skip_all, fields(tx.id = ?entry.id, tx.weight = entry.weight, total_weight = builder.total_weight / max_total_weight))]
fn consider_transaction(
    context: &BlockchainContext,
    max_total_weight: usize,
    entry: &TxEntry,
    tx: &TransactionVerificationData,
    tx_ring_members_info: &TxRingMembersInfo,
    builder: &mut BlockTemplateBuilder,
) -> Option<TransactionHash> {
    // later TODO: check if pruned, continue if it is

    if entry.weight > max_total_weight {
        tracing::info!("exceeds maximum block weight");
        return None;
    }

    let coinbase = if context.current_hf >= HardFork::V5 {
        if builder.total_weight + entry.weight > 2 * context.median_weight_for_block_reward {
            tracing::info!("exceeds maximum block weight");
            return None;
        }

        let block_reward = calculate_block_reward(
            builder.total_weight + entry.weight,
            context.median_weight_for_block_reward,
            context.already_generated_coins,
            context.current_hf,
        );
        let coinbase = block_reward + builder.fee + entry.fee;
        if coinbase < builder.best_coinbase * ACCEPT_THRESHOLD {
            return None;
        }
        Some(coinbase)
    } else if builder.total_weight > context.median_weight_for_block_reward {
        tracing::info!("exceeds median block weight");
        return None;
    } else {
        // TODO: coinbase isn't updated if hf < V5. this is also a minor bug in monerod as far as I've seen it
        None
    };

    let ready = check_transaction_contextual(
        &tx.tx,
        tx_ring_members_info,
        context.chain_height,
        context.current_adjusted_timestamp_for_time_lock(),
        context.current_hf,
    )
    .is_ok()
        && check_transaction_semantic(
            &tx.tx,
            tx.tx_blob.len(),
            tx.tx_weight,
            &tx.tx_hash,
            context.current_hf,
            &MultiThreadedBatchVerifier::new(1),
        )
        .is_ok();

    if !ready {
        tracing::info!("not ready to go");
        return None;
    }

    builder.total_weight += entry.weight;
    builder.fee += entry.fee;
    if let Some(coinbase) = coinbase {
        builder.best_coinbase = coinbase;
    }

    Some(entry.id)
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

fn create_miner_tx(
    context: &BlockchainContext,
    miner_address: &MoneroAddress,
    extra_nonce: &HexVec,
    total_weight: usize,
    fee: u64,
) -> Result<Transaction, Error> {
    //  two-phase miner transaction generation: we don't know exact block weight until we prepare block, but we don't know reward until we know
    //  block weight, so first miner transaction generated with fake amount of money, and with phase we know think we know expected block weight
    let miner_tx = construct_miner_tx(context, miner_address, extra_nonce, total_weight, fee)?;
    let cum_weight = total_weight + miner_tx.weight();

    let mut miner_tx = None;
    for _ in 0..10 {
        let mut found_miner_tx =
            construct_miner_tx(context, miner_address, extra_nonce, cum_weight, fee)?;

        if found_miner_tx.weight() > cum_weight - total_weight {
            tracing::warn!("Miner tx weight is greater than before.");
            continue;
        } else if found_miner_tx.weight() < cum_weight - total_weight {
            let delta = cum_weight - total_weight - found_miner_tx.weight();
            tracing::info!("Miner tx weight is less than before, adding {delta} zero-bytes.");

            found_miner_tx.prefix_mut().extra.put_bytes(0, delta);

            // later TODO: the extra fields may contain `VarInt`s (e.g. via the extra_nonce) which can make a difference now that we've added padding bytes
            // since we don't support extra_nonces yet, this is not an issue for now.
            // <https://github.com/monero-project/monero/blob/8308dee67eba7c6b63ab748834e58c17c5ba4688/src/cryptonote_core/blockchain.cpp#L1763-L1775>
        }

        assert_eq!(cum_weight, total_weight + found_miner_tx.weight());

        miner_tx = Some(found_miner_tx);
        break;
    }
    miner_tx.ok_or_else(|| anyhow::anyhow!("Couldn't construct a miner_tx."))
}

/// constructs miner tx for a block
/// the miner tx pays the block reward (base emission plus the collected `fee`) to `miner_address`
///
/// Note: use [`create_miner_tx`] if you don't know the final weight yet, as the reward depends on `block_weight`.
///
/// ref <https://github.com/monero-project/monero/blob/160e21504aed2a9b6dfdba0970517161383b04e5/src/cryptonote_core/cryptonote_tx_utils.cpp#L105-L208>
fn construct_miner_tx(
    context: &BlockchainContext,
    miner_address: &MoneroAddress,
    extra_nonce: &HexVec,
    block_weight: usize,
    fee: u64,
) -> Result<Transaction, Error> {
    let tx_key = generate_key_pair();

    let block_reward = calculate_block_reward(
        block_weight,
        context.median_weight_for_block_reward,
        context.already_generated_coins,
        context.current_hf,
    ) + fee;
    // later TODO: cut least significant digits from hf >= 2?

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
        assert!(out_amounts.len() <= max_outs, "max_out exceeded");
    }

    let outputs = out_amounts
        .iter()
        .enumerate()
        .map(|(i, amount)| {
            let derivation = generate_key_derivation(miner_address.view(), tx_key.private);
            let out_eph_public_key = derive_public_key(derivation, i, miner_address.spend())?;

            let view_tag = if context.current_hf >= HardFork::V15 {
                Some(derive_view_tag(derivation, i)?)
            } else {
                None
            };
            Ok(Output {
                amount: Some(*amount),
                key: out_eph_public_key.compress(),
                view_tag,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let summary_amounts = out_amounts
        .into_iter()
        .reduce(|acc, e| acc + e)
        .unwrap_or(0);
    assert_eq!(summary_amounts, block_reward);

    let prefix = TransactionPrefix {
        additional_timelock: Timelock::Block(context.chain_height + MINER_TX_TIME_LOCKED_BLOCKS),
        inputs: vec![Input::Gen(context.chain_height)],
        outputs,
        extra: get_extra_field(tx_key.public, extra_nonce),
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
/// ref <https://github.com/monero-project/monero/blob/160e21504aed2a9b6dfdba0970517161383b04e5/src/cryptonote_core/cryptonote_tx_utils.cpp#L110-L116>
fn get_extra_field(public_key: CompressedPoint, extra_nonce: &HexVec) -> Vec<u8> {
    assert!(
        extra_nonce.len() <= TX_EXTRA_NONCE_MAX_COUNT,
        "extra nonce could be 255 bytes max"
    );
    let mut extra = Vec::with_capacity(33 + extra_nonce.len());

    extra.put_u8(TX_EXTRA_TAG_PUBKEY);
    extra.extend_from_slice(&public_key.to_bytes());

    if !extra_nonce.is_empty() {
        // use var in to write it into the buf
        // <https://github.com/monero-project/monero/blob/8308dee67eba7c6b63ab748834e58c17c5ba4688/src/cryptonote_basic/cryptonote_format_utils.cpp#L869-L887>
        unimplemented!("extra nonce is not used by monero-oxide");
    }
    // sorting doesn't have to be done here as there are just these 2 fields (which are in order)

    extra
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
        order *= 10;

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
    let view = view.into();
    let private = private.into();
    Point::from((view * private).mul_by_cofactor())
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
