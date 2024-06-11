use std::{
    cmp,
    collections::HashMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[allow(unused_imports)]
use hex_literal::hex;
use monero_serai::{
    block::Block,
    transaction::{Input, Transaction}
};
use tower::Service;

use cuprate_consensus_rules::{
    blocks::BlockError,
    miner_tx::{check_miner_tx, MinerTxError},
    ConsensusError,
};
use cuprate_types::VerifiedBlockInformation;
use cuprate_types::VerifiedTransactionInformation;

use crate::{hash_of_hashes, BlockId, HashOfHashes};

#[cfg(not(test))]
static HASHES_OF_HASHES: &[HashOfHashes] = &include!("./data/hashes_of_hashes");

#[cfg(not(test))]
const BATCH_SIZE: usize = 512;

#[cfg(test)]
static HASHES_OF_HASHES: &[HashOfHashes] = &[
    hex!("3fdc9032c16d440f6c96be209c36d3d0e1aed61a2531490fe0ca475eb615c40a"),
    hex!("0102030405060708010203040506070801020304050607080102030405060708"),
    hex!("0102030405060708010203040506070801020304050607080102030405060708"),
];

#[cfg(test)]
const BATCH_SIZE: usize = 4;

#[inline]
fn max_height() -> u64 {
    (HASHES_OF_HASHES.len() * BATCH_SIZE) as u64
}

#[derive(Debug, PartialEq)]
pub struct ValidBlockId(BlockId);

fn valid_block_ids(block_ids: &[BlockId]) -> Vec<ValidBlockId> {
    block_ids.iter().map(|b| ValidBlockId(*b)).collect()
}

pub enum FastSyncRequest {
    ValidateHashes {
        start_height: u64,
        block_ids: Vec<BlockId>,
    },
    ValidateBlock {
        block: Block,
        txs: HashMap<[u8; 32], Transaction>,
        token: ValidBlockId,
    },
}

#[derive(Debug, PartialEq)]
pub enum FastSyncResponse {
    ValidateHashes {
        validated_hashes: Vec<ValidBlockId>,
        unknown_hashes: Vec<BlockId>,
    },
    ValidateBlock(VerifiedBlockInformation),
}

#[derive(Debug, PartialEq)]
pub enum FastSyncError {
    BlockHashMismatch,  // block does not match its expected hash
    InvalidStartHeight, // start_height not a multiple of BATCH_SIZE
    Mismatch,           // hash of hashes for one batch does not match
    NothingToDo,        // no complete batch to check
    OutOfRange,         // start_height too high
}

#[allow(dead_code)]
pub struct FastSyncService<C> {
    context_svc: C,
}

impl<C> FastSyncService<C>
where
    C: Service<FastSyncRequest, Response = FastSyncResponse, Error = FastSyncError>
        + Clone
        + Send
        + 'static,
{
    #[allow(dead_code)]
    pub(crate) fn new(context_svc: C) -> FastSyncService<C> {
        FastSyncService { context_svc }
    }
}

impl<C> Service<FastSyncRequest> for FastSyncService<C>
where
    C: Service<FastSyncRequest, Response = FastSyncResponse, Error = FastSyncError>
        + Clone
        + Send
        + 'static,
    C::Future: Send + 'static,
{
    type Response = FastSyncResponse;
    type Error = FastSyncError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: FastSyncRequest) -> Self::Future {
        Box::pin(async move {
            match req {
                FastSyncRequest::ValidateHashes {
                    start_height,
                    block_ids,
                } => validate_hashes(start_height, &block_ids).await,
                FastSyncRequest::ValidateBlock {
                    block,
                    txs,
                    token,
                } => validate_block(block, txs, token).await,
            }
        })
    }
}

async fn validate_hashes(
    start_height: u64,
    block_ids: &[BlockId],
) -> Result<FastSyncResponse, FastSyncError> {
    if start_height as usize % BATCH_SIZE != 0 {
        return Err(FastSyncError::InvalidStartHeight);
    }

    if start_height >= max_height() {
        return Err(FastSyncError::OutOfRange);
    }

    let stop_height = start_height as usize + block_ids.len();

    let batch_from = start_height as usize / BATCH_SIZE;
    let batch_to = cmp::min(stop_height / BATCH_SIZE, HASHES_OF_HASHES.len());
    let n_batches = batch_to - batch_from;

    if n_batches == 0 {
        return Err(FastSyncError::NothingToDo);
    }

    for i in 0..n_batches {
        let batch = &block_ids[BATCH_SIZE * i..BATCH_SIZE * (i + 1)];
        let actual = hash_of_hashes(batch);
        let expected = HASHES_OF_HASHES[batch_from + i];

        if expected != actual {
            return Err(FastSyncError::Mismatch);
        }
    }

    let validated_hashes = valid_block_ids(&block_ids[..n_batches * BATCH_SIZE]);
    let unknown_hashes = block_ids[n_batches * BATCH_SIZE..].to_vec();

    Ok(FastSyncResponse::ValidateHashes {
        validated_hashes,
        unknown_hashes,
    })
}

async fn validate_block(
    block: Block,
    txs: HashMap<[u8; 32], Transaction>,
    token: ValidBlockId,
) -> Result<FastSyncResponse, FastSyncError>
{
    let block_hash = block.hash();
    if block_hash != token.0 {
        return Err(FastSyncError::BlockHashMismatch)
    }

    let block_blob = block.serialize();
    let txs_vec: Vec<Transaction> = txs.values().cloned().collect();
    let verifi_data_txs: Vec<TransactionVerificationData> = txs_vec.into_iter()
        .map(|tx| {
            TransactionVerificationData(tx)
        }).collect();
    let Some(Input::Gen(height)) = block.miner_tx.prefix.inputs.first() else {

        Err(ConsensusError::Block(BlockError::MinerTxError(
            MinerTxError::InputNotOfTypeGen,
        )))?
    };

    let total_fees = txs.iter().map(|tx| tx.fee).sum::<u64>();

    let generated_coins = check_miner_tx(
        &block.miner_tx,
        total_fees,
        block_chain_ctx.chain_height,
        block_weight,
        block_chain_ctx.median_weight_for_block_reward,
        block_chain_ctx.already_generated_coins,
        &block_chain_ctx.current_hf,
    )?;

    Ok(FastSyncResponse::ValidateBlock(VerifiedBlockInformation {
        block,
        block_blob,
        txs: vec![],
        block_hash,
        pow_hash: [0u8; 32],
        height: *height,
        generated_coins: 0u64,
        weight: 0usize,
        long_term_weight: 0usize,
        cumulative_difficulty: 0u128,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;

    #[test]
    fn test_validate_hashes_errors() {
        let ids = [[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32]];
        assert_eq!(
            block_on(validate_hashes(3, &[])),
            Err(FastSyncError::InvalidStartHeight)
        );
        assert_eq!(
            block_on(validate_hashes(3, &ids)),
            Err(FastSyncError::InvalidStartHeight)
        );

        assert_eq!(
            block_on(validate_hashes(20, &[])),
            Err(FastSyncError::OutOfRange)
        );
        assert_eq!(
            block_on(validate_hashes(20, &ids)),
            Err(FastSyncError::OutOfRange)
        );

        assert_eq!(
            block_on(validate_hashes(4, &[])),
            Err(FastSyncError::NothingToDo)
        );
        assert_eq!(
            block_on(validate_hashes(4, &ids[..3])),
            Err(FastSyncError::NothingToDo)
        );
    }

    #[test]
    fn test_validate_hashes_success() {
        let ids = [[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32]];
        let validated_hashes = valid_block_ids(&ids[0..4]);
        let unknown_hashes = ids[4..].to_vec();
        assert_eq!(
            block_on(validate_hashes(0, &ids)),
            Ok(FastSyncResponse::ValidateHashes {
                validated_hashes,
                unknown_hashes
            })
        );
    }

    #[test]
    fn test_validate_hashes_mismatch() {
        let ids = [
            [1u8; 32], [2u8; 32], [3u8; 32], [5u8; 32], [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32],
        ];
        assert_eq!(
            block_on(validate_hashes(0, &ids)),
            Err(FastSyncError::Mismatch)
        );
        assert_eq!(
            block_on(validate_hashes(4, &ids)),
            Err(FastSyncError::Mismatch)
        );
    }
}
