use std::{
    collections::HashSet,
    future::{ready, Ready},
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{future::BoxFuture, FutureExt};
use monero_serai::{block::Block, transaction::Transaction};
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::{transactions::new_tx_verification_data, BlockChainContextService};
use cuprate_fixed_bytes::ByteArrayVec;
use cuprate_helper::{
    asynch::rayon_spawn_async,
    cast::usize_to_u64,
    map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits},
};
use cuprate_p2p::constants::MAX_BLOCK_BATCH_LEN;
use cuprate_p2p_core::{client::PeerInformation, NetworkZone, ProtocolRequest, ProtocolResponse};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    BlockCompleteEntry, MissingTxsInBlock, TransactionBlobs,
};
use cuprate_wire::protocol::{
    ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, GetObjectsRequest,
    GetObjectsResponse, NewFluffyBlock,
};

use crate::blockchain::interface::{self as blockchain_interface, IncomingBlockError};

/// The P2P protocol request handler [`MakeService`](tower::MakeService).
#[derive(Clone)]
pub struct P2pProtocolRequestHandlerMaker {
    /// The [`BlockchainReadHandle`]
    pub blockchain_read_handle: BlockchainReadHandle,
}

impl<N: NetworkZone> Service<PeerInformation<N>> for P2pProtocolRequestHandlerMaker {
    type Response = P2pProtocolRequestHandler<N>;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, peer_information: PeerInformation<N>) -> Self::Future {
        // TODO: check sync info?

        let blockchain_read_handle = self.blockchain_read_handle.clone();

        ready(Ok(P2pProtocolRequestHandler {
            peer_information,
            blockchain_read_handle,
        }))
    }
}

/// The P2P protocol request handler.
#[derive(Clone)]
pub struct P2pProtocolRequestHandler<N: NetworkZone> {
    /// The [`PeerInformation`] for this peer.
    peer_information: PeerInformation<N>,
    /// The [`BlockchainReadHandle`]
    blockchain_read_handle: BlockchainReadHandle,
}

impl<Z: NetworkZone> Service<ProtocolRequest> for P2pProtocolRequestHandler<Z> {
    type Response = ProtocolResponse;
    type Error = anyhow::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: ProtocolRequest) -> Self::Future {
        match request {
            ProtocolRequest::GetObjects(r) => {
                get_objects(r, self.blockchain_read_handle.clone()).boxed()
            }
            ProtocolRequest::GetChain(r) => {
                get_chain(r, self.blockchain_read_handle.clone()).boxed()
            }
            ProtocolRequest::FluffyMissingTxs(r) => {
                fluffy_missing_txs(r, self.blockchain_read_handle.clone()).boxed()
            }
            ProtocolRequest::NewBlock(_) => ready(Err(anyhow::anyhow!(
                "Peer sent a full block when we support fluffy blocks"
            )))
            .boxed(),
            ProtocolRequest::NewFluffyBlock(r) => {
                new_fluffy_block(r, self.blockchain_read_handle.clone()).boxed()
            }
            ProtocolRequest::GetTxPoolCompliment(_) | ProtocolRequest::NewTransactions(_) => {
                ready(Ok(ProtocolResponse::NA)).boxed()
            } // TODO: tx-pool
        }
    }
}

//---------------------------------------------------------------------------------------------------- Handler functions

/// [`ProtocolRequest::GetObjects`]
async fn get_objects(
    request: GetObjectsRequest,
    mut blockchain_read_handle: BlockchainReadHandle,
) -> anyhow::Result<ProtocolResponse> {
    if request.blocks.len() > MAX_BLOCK_BATCH_LEN {
        anyhow::bail!("Peer requested more blocks than allowed.")
    }

    let block_hashes: Vec<[u8; 32]> = (&request.blocks).into();
    // deallocate the backing `Bytes`.
    drop(request);

    let BlockchainResponse::BlockCompleteEntries {
        blocks,
        missing_hashes,
        blockchain_height,
    } = blockchain_read_handle
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockCompleteEntries(block_hashes))
        .await?
    else {
        panic!("blockchain returned wrong response!");
    };

    Ok(ProtocolResponse::GetObjects(GetObjectsResponse {
        blocks,
        missed_ids: ByteArrayVec::from(missing_hashes),
        current_blockchain_height: usize_to_u64(blockchain_height),
    }))
}

/// [`ProtocolRequest::GetChain`]
async fn get_chain(
    request: ChainRequest,
    mut blockchain_read_handle: BlockchainReadHandle,
) -> anyhow::Result<ProtocolResponse> {
    if request.block_ids.len() > 25_000 {
        anyhow::bail!("Peer sent too many block hashes in chain request.")
    }

    let block_hashes: Vec<[u8; 32]> = (&request.block_ids).into();
    let want_pruned_data = request.prune;
    // deallocate the backing `Bytes`.
    drop(request);

    let BlockchainResponse::NextChainEntry {
        start_height,
        chain_height,
        block_ids,
        block_weights,
        cumulative_difficulty,
        first_block_blob,
    } = blockchain_read_handle
        .ready()
        .await?
        .call(BlockchainReadRequest::NextChainEntry(block_hashes, 10_000))
        .await?
    else {
        panic!("blockchain returned wrong response!");
    };

    if start_height == 0 {
        anyhow::bail!("The peers chain has a different genesis block than ours.");
    }

    let (cumulative_difficulty_low64, cumulative_difficulty_top64) =
        split_u128_into_low_high_bits(cumulative_difficulty);

    Ok(ProtocolResponse::GetChain(ChainResponse {
        start_height: usize_to_u64(start_height),
        total_height: usize_to_u64(chain_height),
        cumulative_difficulty_low64,
        cumulative_difficulty_top64,
        m_block_ids: ByteArrayVec::from(block_ids),
        first_block: first_block_blob.map_or(Bytes::new(), Bytes::from),
        // only needed when
        m_block_weights: if want_pruned_data {
            block_weights.into_iter().map(usize_to_u64).collect()
        } else {
            vec![]
        },
    }))
}

/// [`ProtocolRequest::FluffyMissingTxs`]
async fn fluffy_missing_txs(
    mut request: FluffyMissingTransactionsRequest,
    mut blockchain_read_handle: BlockchainReadHandle,
) -> anyhow::Result<ProtocolResponse> {
    let tx_indexes = std::mem::take(&mut request.missing_tx_indices);
    let block_hash: [u8; 32] = *request.block_hash;
    let current_blockchain_height = request.current_blockchain_height;

    // deallocate the backing `Bytes`.
    drop(request);

    let BlockchainResponse::MissingTxsInBlock(res) = blockchain_read_handle
        .ready()
        .await?
        .call(BlockchainReadRequest::MissingTxsInBlock {
            block_hash,
            tx_indexes,
        })
        .await?
    else {
        panic!("blockchain returned wrong response!");
    };

    let Some(MissingTxsInBlock { block, txs }) = res else {
        anyhow::bail!("The peer requested txs out of range.");
    };

    Ok(ProtocolResponse::NewFluffyBlock(NewFluffyBlock {
        b: BlockCompleteEntry {
            block: Bytes::from(block),
            txs: TransactionBlobs::Normal(txs.into_iter().map(Bytes::from).collect()),
            pruned: false,
            // only needed for pruned blocks.
            block_weight: 0,
        },
        current_blockchain_height,
    }))
}

/// [`ProtocolRequest::NewFluffyBlock`]
async fn new_fluffy_block(
    request: NewFluffyBlock,
    mut blockchain_read_handle: BlockchainReadHandle,
) -> anyhow::Result<ProtocolResponse> {
    let current_blockchain_height = request.current_blockchain_height;

    let (block, txs) = rayon_spawn_async(move || -> Result<_, anyhow::Error> {
        let block = Block::read(&mut request.b.block.as_ref())?;

        let tx_blobs = request
            .b
            .txs
            .take_normal()
            .ok_or(anyhow::anyhow!("Peer sent pruned txs in fluffy block"))?;

        // TODO: size check these tx blobs
        let txs = tx_blobs
            .into_iter()
            .map(|tx_blob| {
                let tx = Transaction::read(&mut tx_blob.as_ref())?;

                Ok(tx)
            })
            .collect::<Result<_, anyhow::Error>>()?;

        // The backing `Bytes` will be deallocated when this closure returns.

        Ok((block, txs))
    })
    .await?;

    let res =
        blockchain_interface::handle_incoming_block(block, txs, &mut blockchain_read_handle).await;

    match res {
        Ok(_) => Ok(ProtocolResponse::NA),
        Err(IncomingBlockError::UnknownTransactions(block_hash, missing_tx_indices)) => Ok(
            ProtocolResponse::FluffyMissingTransactionsRequest(FluffyMissingTransactionsRequest {
                block_hash: block_hash.into(),
                current_blockchain_height,
                missing_tx_indices,
            }),
        ),
        Err(IncomingBlockError::Orphan) => {
            // Block's parent was unknown, could be syncing?
            Ok(ProtocolResponse::NA)
        }
        Err(e) => Err(e.into()),
    }
}
