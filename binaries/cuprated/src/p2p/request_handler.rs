use bytes::Bytes;
use cuprate_p2p_core::{ProtocolRequest, ProtocolResponse};
use futures::future::BoxFuture;
use futures::FutureExt;
use monero_serai::block::Block;
use monero_serai::transaction::Transaction;
use rayon::prelude::*;
use std::task::{Context, Poll};
use tower::{Service, ServiceExt};
use tracing::trace;

use crate::blockchain::{handle_incoming_block, IncomingBlockError};
use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_fixed_bytes::ByteArray;
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_helper::cast::usize_to_u64;
use cuprate_helper::map::split_u128_into_low_high_bits;
use cuprate_p2p::constants::{MAX_BLOCKCHAIN_SUPPLEMENT_LEN, MAX_BLOCK_BATCH_LEN};
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};
use cuprate_types::BlockCompleteEntry;
use cuprate_wire::protocol::{ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, GetObjectsRequest, GetObjectsResponse, NewFluffyBlock};

#[derive(Clone)]
pub struct P2pProtocolRequestHandler {
    pub(crate) blockchain_read_handle: BlockchainReadHandle,
}

impl Service<ProtocolRequest> for P2pProtocolRequestHandler {
    type Response = ProtocolResponse;
    type Error = tower::BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ProtocolRequest) -> Self::Future {
        match req {
            ProtocolRequest::GetObjects(req) => {
                get_objects(self.blockchain_read_handle.clone(), req).boxed()
            }
            ProtocolRequest::GetChain(req) => {
                get_chain(self.blockchain_read_handle.clone(), req).boxed()
            }
            ProtocolRequest::FluffyMissingTxs(_) => async { Ok(ProtocolResponse::NA) }.boxed(),
            ProtocolRequest::GetTxPoolCompliment(_) => async { Ok(ProtocolResponse::NA) }.boxed(),
            ProtocolRequest::NewBlock(_) => async { Ok(ProtocolResponse::NA) }.boxed(),
            ProtocolRequest::NewFluffyBlock(block) => new_fluffy_block(self.blockchain_read_handle.clone(), block).boxed(),
            ProtocolRequest::NewTransactions(_) => async { Ok(ProtocolResponse::NA) }.boxed(),
        }
    }
}

async fn get_objects(
    blockchain_read_handle: BlockchainReadHandle,
    req: GetObjectsRequest,
) -> Result<ProtocolResponse, tower::BoxError> {
    if req.blocks.is_empty() {
        Err("No blocks requested in a GetObjectsRequest")?;
    }

    if req.blocks.len() > MAX_BLOCK_BATCH_LEN {
        Err("Too many blocks requested in a GetObjectsRequest")?;
    }

    let block_ids: Vec<[u8; 32]> = (&req.blocks).into();
    // de-allocate the backing [`Bytes`]
    drop(req);

    let res = blockchain_read_handle
        .oneshot(BlockchainReadRequest::BlockCompleteEntries(block_ids))
        .await?;

    let BlockchainResponse::BlockCompleteEntries {
        blocks,
        missed_ids,
        current_blockchain_height,
    } = res
    else {
        panic!("Blockchain service returned wrong response!");
    };

    Ok(ProtocolResponse::GetObjects(GetObjectsResponse {
        blocks,
        missed_ids: missed_ids.into(),
        current_blockchain_height: usize_to_u64(current_blockchain_height),
    }))
}

async fn get_chain(
    blockchain_read_handle: BlockchainReadHandle,
    req: ChainRequest,
) -> Result<ProtocolResponse, tower::BoxError> {
    if req.block_ids.is_empty() {
        Err("No block hashes sent in a `ChainRequest`")?;
    }

    if req.block_ids.len() > MAX_BLOCKCHAIN_SUPPLEMENT_LEN {
        Err("Too many block hashes in a `ChainRequest`")?;
    }

    let block_ids: Vec<[u8; 32]> = (&req.block_ids).into();
    // de-allocate the backing [`Bytes`]
    drop(req);

    let res = blockchain_read_handle
        .oneshot(BlockchainReadRequest::NextMissingChainEntry(block_ids))
        .await?;

    let BlockchainResponse::NextMissingChainEntry {
        next_entry,
        first_missing_block,
        start_height,
        chain_height,
        cumulative_difficulty,
    } = res
    else {
        panic!("Blockchain service returned wrong response!");
    };

    let (cumulative_difficulty_low64, cumulative_difficulty_top64) =
        split_u128_into_low_high_bits(cumulative_difficulty);

    Ok(ProtocolResponse::GetChain(ChainResponse {
        start_height: usize_to_u64(start_height),
        total_height: usize_to_u64(chain_height),
        cumulative_difficulty_low64,
        cumulative_difficulty_top64,
        m_block_ids: next_entry.into(),
        m_block_weights: vec![],
        first_block: first_missing_block.map_or(Bytes::new(), Bytes::from),
    }))
}

async fn new_fluffy_block(
    mut blockchain_read_handle: BlockchainReadHandle,
    incoming_block: NewFluffyBlock,
) -> Result<ProtocolResponse, tower::BoxError> {
    let peer_blockchain_height = incoming_block.current_blockchain_height;

    let (block, txs) = rayon_spawn_async(move || {
        let block = Block::read(&mut incoming_block.b.block.as_ref())?;
        let txs = incoming_block
            .b
            .txs
            .take_normal()
            .expect("TODO")
            .into_par_iter()
            .map(|tx| {
                let tx = Transaction::read(&mut tx.as_ref())?;
                Ok(tx)
            })
            .collect::<Result<_, tower::BoxError>>()?;

        Ok::<_, tower::BoxError>((block, txs))
    })
    .await?;

    let res = handle_incoming_block(block, txs, &mut blockchain_read_handle).await;
    
    match res { 
        Err(IncomingBlockError::UnknownTransactions(block_hash, tx_indexes)) => {
            return Ok(ProtocolResponse::FluffyMissingTxs(FluffyMissingTransactionsRequest{
                block_hash: ByteArray::from(block_hash),
                current_blockchain_height: peer_blockchain_height,
                missing_tx_indices: tx_indexes,
            }))
        }
        Err(IncomingBlockError::InvalidBlock(e)) => Err(e)?,
        Err(IncomingBlockError::Orphan) | Ok(_) => Ok(ProtocolResponse::NA),
    }
}
