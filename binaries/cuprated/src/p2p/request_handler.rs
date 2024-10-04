use bytes::Bytes;
use futures::future::BoxFuture;
use futures::FutureExt;
use std::{
    future::{ready, Ready},
    task::{Context, Poll},
};
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::BlockChainContextService;
use cuprate_fixed_bytes::ByteArrayVec;
use cuprate_helper::cast::usize_to_u64;
use cuprate_helper::map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits};
use cuprate_p2p::constants::MAX_BLOCK_BATCH_LEN;
use cuprate_p2p_core::{client::PeerInformation, NetworkZone, ProtocolRequest, ProtocolResponse};
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};
use cuprate_types::{BlockCompleteEntry, MissingTxsInBlock, TransactionBlobs};
use cuprate_wire::protocol::{
    ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, GetObjectsRequest,
    GetObjectsResponse, NewFluffyBlock,
};

#[derive(Clone)]
pub struct P2pProtocolRequestHandlerMaker {
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

#[derive(Clone)]
pub struct P2pProtocolRequestHandler<N: NetworkZone> {
    peer_information: PeerInformation<N>,
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
            ProtocolRequest::NewFluffyBlock(_) => todo!(),
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
    // de-allocate the backing `Bytes`.
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
    // de-allocate the backing `Bytes`.
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

    // de-allocate the backing `Bytes`.
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
