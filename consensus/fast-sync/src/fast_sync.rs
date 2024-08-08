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
    transaction::{Input, Transaction},
};
use tower::{Service, ServiceExt};

use cuprate_consensus::{
    context::{BlockChainContextRequest, BlockChainContextResponse},
    transactions::new_tx_verification_data,
};
use cuprate_consensus_rules::{miner_tx::MinerTxError, ConsensusError};
use cuprate_types::{VerifiedBlockInformation, VerifiedTransactionInformation};

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

#[allow(clippy::large_enum_variant)]
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

#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq)]
pub enum FastSyncResponse {
    ValidateHashes {
        validated_hashes: Vec<ValidBlockId>,
        unknown_hashes: Vec<BlockId>,
    },
    ValidateBlock(VerifiedBlockInformation),
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum FastSyncError {
    #[error("Block does not match its expected hash")]
    BlockHashMismatch,

    #[error("Start height must be a multiple of the batch size")]
    InvalidStartHeight,

    #[error("Hash of hashes mismatch")]
    Mismatch,

    #[error("Given range too small for fast sync (less than one batch)")]
    NothingToDo,

    #[error("Start height too high for fast sync")]
    OutOfRange,

    #[error("Block does not have the expected height entry")]
    BlockHeightMismatch,

    #[error("Block does not contain the expected transaction list")]
    TxsIncludedWithBlockIncorrect,

    #[error(transparent)]
    Consensus(#[from] ConsensusError),

    #[error(transparent)]
    MinerTx(#[from] MinerTxError),

    #[error("Database error: {0}")]
    DbErr(String),
}

impl From<tower::BoxError> for FastSyncError {
    fn from(error: tower::BoxError) -> Self {
        Self::DbErr(error.to_string())
    }
}

pub struct FastSyncService<C> {
    context_svc: C,
}

impl<C> FastSyncService<C>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Clone
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
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Clone
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
        let context_svc = self.context_svc.clone();

        Box::pin(async move {
            match req {
                FastSyncRequest::ValidateHashes {
                    start_height,
                    block_ids,
                } => validate_hashes(start_height, &block_ids).await,
                FastSyncRequest::ValidateBlock { block, txs, token } => {
                    validate_block(context_svc, block, txs, token).await
                }
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

async fn validate_block<C>(
    mut context_svc: C,
    block: Block,
    mut txs: HashMap<[u8; 32], Transaction>,
    token: ValidBlockId,
) -> Result<FastSyncResponse, FastSyncError>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Send
        + 'static,
    C::Future: Send + 'static,
{
    let BlockChainContextResponse::Context(checked_context) = context_svc
        .ready()
        .await?
        .call(BlockChainContextRequest::GetContext)
        .await?
    else {
        panic!("Context service returned wrong response!");
    };

    let block_chain_ctx = checked_context.unchecked_blockchain_context().clone();

    let block_hash = block.hash();
    if block_hash != token.0 {
        return Err(FastSyncError::BlockHashMismatch);
    }

    let block_blob = block.serialize();

    let Some(Input::Gen(height)) = block.miner_transaction.prefix().inputs.first() else {
        return Err(FastSyncError::MinerTx(MinerTxError::InputNotOfTypeGen));
    };
    if *height != block_chain_ctx.chain_height {
        return Err(FastSyncError::BlockHeightMismatch);
    }

    let mut verified_txs = Vec::with_capacity(txs.len());
    for tx in &block.transactions {
        let tx = txs
            .remove(tx)
            .ok_or(FastSyncError::TxsIncludedWithBlockIncorrect)?;

        let data = new_tx_verification_data(tx)?;
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

    Ok(FastSyncResponse::ValidateBlock(VerifiedBlockInformation {
        block_blob,
        txs: verified_txs,
        block_hash,
        pow_hash: [0u8; 32],
        height: *height,
        generated_coins,
        weight,
        long_term_weight: block_chain_ctx.next_block_long_term_weight(weight),
        cumulative_difficulty: block_chain_ctx.cumulative_difficulty
            + block_chain_ctx.next_difficulty,
        block,
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
