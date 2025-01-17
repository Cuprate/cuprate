use std::{
    collections::HashSet,
    future::{ready, Ready},
    hash::Hash,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{
    future::{BoxFuture, Shared},
    FutureExt,
};
use monero_serai::{block::Block, transaction::Transaction};
use tokio::sync::{broadcast, oneshot, watch};
use tokio_stream::wrappers::WatchStream;
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::{
    transactions::new_tx_verification_data, BlockChainContextRequest, BlockChainContextResponse,
    BlockchainContextService,
};
use cuprate_dandelion_tower::TxState;
use cuprate_fixed_bytes::ByteArrayVec;
use cuprate_helper::cast::u64_to_usize;
use cuprate_helper::{
    asynch::rayon_spawn_async,
    cast::usize_to_u64,
    map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits},
};
use cuprate_p2p::constants::{
    MAX_BLOCKS_IDS_IN_CHAIN_ENTRY, MAX_BLOCK_BATCH_LEN, MAX_TRANSACTION_BLOB_SIZE, MEDIUM_BAN,
};
use cuprate_p2p_core::{
    client::{InternalPeerID, PeerInformation},
    NetZoneAddress, NetworkZone, ProtocolRequest, ProtocolResponse,
};
use cuprate_txpool::service::TxpoolReadHandle;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    BlockCompleteEntry, TransactionBlobs, TxsInBlock,
};
use cuprate_wire::protocol::{
    ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, GetObjectsRequest,
    GetObjectsResponse, NewFluffyBlock, NewTransactions,
};

use crate::{
    blockchain::interface::{self as blockchain_interface, IncomingBlockError},
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    p2p::CrossNetworkInternalPeerId,
    txpool::{IncomingTxError, IncomingTxHandler, IncomingTxs},
};

/// The P2P protocol request handler [`MakeService`](tower::MakeService).
#[derive(Clone)]
pub struct P2pProtocolRequestHandlerMaker {
    pub blockchain_read_handle: BlockchainReadHandle,
    pub blockchain_context_service: BlockchainContextService,
    pub txpool_read_handle: TxpoolReadHandle,

    /// The [`IncomingTxHandler`], wrapped in an [`Option`] as there is a cyclic reference between [`P2pProtocolRequestHandlerMaker`]
    /// and the [`IncomingTxHandler`].
    pub incoming_tx_handler: Option<IncomingTxHandler>,

    /// A [`Future`](std::future::Future) that produces the [`IncomingTxHandler`].
    pub incoming_tx_handler_fut: Shared<oneshot::Receiver<IncomingTxHandler>>,
}

impl<A: NetZoneAddress> Service<PeerInformation<A>> for P2pProtocolRequestHandlerMaker
where
    InternalPeerID<A>: Into<CrossNetworkInternalPeerId>,
{
    type Response = P2pProtocolRequestHandler<A>;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.incoming_tx_handler.is_none() {
            return self
                .incoming_tx_handler_fut
                .poll_unpin(cx)
                .map(|incoming_tx_handler| {
                    self.incoming_tx_handler = Some(incoming_tx_handler?);
                    Ok(())
                });
        }

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, peer_information: PeerInformation<A>) -> Self::Future {
        let Some(incoming_tx_handler) = self.incoming_tx_handler.clone() else {
            panic!("poll_ready was not called or did not return `Poll::Ready`")
        };

        // TODO: check sync info?

        let blockchain_read_handle = self.blockchain_read_handle.clone();
        let txpool_read_handle = self.txpool_read_handle.clone();

        ready(Ok(P2pProtocolRequestHandler {
            peer_information,
            blockchain_read_handle,
            blockchain_context_service: self.blockchain_context_service.clone(),
            txpool_read_handle,
            incoming_tx_handler,
        }))
    }
}

/// The P2P protocol request handler.
#[derive(Clone)]
pub struct P2pProtocolRequestHandler<N: NetZoneAddress> {
    peer_information: PeerInformation<N>,
    blockchain_read_handle: BlockchainReadHandle,
    blockchain_context_service: BlockchainContextService,
    txpool_read_handle: TxpoolReadHandle,
    incoming_tx_handler: IncomingTxHandler,
}

impl<A: NetZoneAddress> Service<ProtocolRequest> for P2pProtocolRequestHandler<A>
where
    InternalPeerID<A>: Into<CrossNetworkInternalPeerId>,
{
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
            ProtocolRequest::NewFluffyBlock(r) => new_fluffy_block(
                self.peer_information.clone(),
                r,
                self.blockchain_read_handle.clone(),
                self.txpool_read_handle.clone(),
            )
            .boxed(),
            ProtocolRequest::NewTransactions(r) => new_transactions(
                self.peer_information.clone(),
                r,
                self.blockchain_context_service.clone(),
                self.incoming_tx_handler.clone(),
            )
            .boxed(),
            ProtocolRequest::GetTxPoolCompliment(_) => ready(Ok(ProtocolResponse::NA)).boxed(), // TODO: should we support this?
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
        unreachable!();
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
    if request.block_ids.len() > MAX_BLOCKS_IDS_IN_CHAIN_ENTRY {
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
        unreachable!();
    };

    let Some(start_height) = start_height else {
        anyhow::bail!("The peers chain has a different genesis block than ours.");
    };

    let (cumulative_difficulty_low64, cumulative_difficulty_top64) =
        split_u128_into_low_high_bits(cumulative_difficulty);

    Ok(ProtocolResponse::GetChain(ChainResponse {
        start_height: usize_to_u64(std::num::NonZero::get(start_height)),
        total_height: usize_to_u64(chain_height),
        cumulative_difficulty_low64,
        cumulative_difficulty_top64,
        m_block_ids: ByteArrayVec::from(block_ids),
        first_block: first_block_blob.map_or(Bytes::new(), Bytes::from),
        // only needed when pruned
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

    let BlockchainResponse::TxsInBlock(res) = blockchain_read_handle
        .ready()
        .await?
        .call(BlockchainReadRequest::TxsInBlock {
            block_hash,
            tx_indexes,
        })
        .await?
    else {
        unreachable!();
    };

    let Some(TxsInBlock { block, txs }) = res else {
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
async fn new_fluffy_block<A: NetZoneAddress>(
    peer_information: PeerInformation<A>,
    request: NewFluffyBlock,
    mut blockchain_read_handle: BlockchainReadHandle,
    mut txpool_read_handle: TxpoolReadHandle,
) -> anyhow::Result<ProtocolResponse> {
    // TODO: check context service here and ignore the block?
    let current_blockchain_height = request.current_blockchain_height;

    peer_information
        .core_sync_data
        .lock()
        .unwrap()
        .current_height = current_blockchain_height;

    let (block, txs) = rayon_spawn_async(move || -> Result<_, anyhow::Error> {
        let block = Block::read(&mut request.b.block.as_ref())?;

        let tx_blobs = request
            .b
            .txs
            .take_normal()
            .ok_or(anyhow::anyhow!("Peer sent pruned txs in fluffy block"))?;

        let txs = tx_blobs
            .into_iter()
            .map(|tx_blob| {
                if tx_blob.len() > MAX_TRANSACTION_BLOB_SIZE {
                    anyhow::bail!("Peer sent a transaction over the size limit.");
                }

                let tx = Transaction::read(&mut tx_blob.as_ref())?;

                Ok((tx.hash(), tx))
            })
            .collect::<Result<_, anyhow::Error>>()?;

        // The backing `Bytes` will be deallocated when this closure returns.

        Ok((block, txs))
    })
    .await?;

    let res = blockchain_interface::handle_incoming_block(
        block,
        txs,
        &mut blockchain_read_handle,
        &mut txpool_read_handle,
    )
    .await;

    match res {
        Ok(_) => Ok(ProtocolResponse::NA),
        Err(IncomingBlockError::UnknownTransactions(block_hash, missing_tx_indices)) => Ok(
            ProtocolResponse::FluffyMissingTransactionsRequest(FluffyMissingTransactionsRequest {
                block_hash: block_hash.into(),
                current_blockchain_height,
                missing_tx_indices: missing_tx_indices.into_iter().map(usize_to_u64).collect(),
            }),
        ),
        Err(IncomingBlockError::Orphan) => {
            // Block's parent was unknown, could be syncing?
            Ok(ProtocolResponse::NA)
        }
        Err(e) => Err(e.into()),
    }
}

/// [`ProtocolRequest::NewTransactions`]
async fn new_transactions<A>(
    peer_information: PeerInformation<A>,
    request: NewTransactions,
    mut blockchain_context_service: BlockchainContextService,
    mut incoming_tx_handler: IncomingTxHandler,
) -> anyhow::Result<ProtocolResponse>
where
    A: NetZoneAddress,
    InternalPeerID<A>: Into<CrossNetworkInternalPeerId>,
{
    let context = blockchain_context_service.blockchain_context();

    // If we are more than 2 blocks behind the peer then ignore the txs - we are probably still syncing.
    if usize_to_u64(context.chain_height + 2)
        < peer_information
            .core_sync_data
            .lock()
            .unwrap()
            .current_height
    {
        return Ok(ProtocolResponse::NA);
    }

    let state = if request.dandelionpp_fluff {
        TxState::Fluff
    } else {
        TxState::Stem {
            from: peer_information.id.into(),
        }
    };

    // Drop all the data except the stuff we still need.
    let NewTransactions { txs, .. } = request;

    let res = incoming_tx_handler
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(IncomingTxs { txs, state })
        .await;

    match res {
        Ok(()) => Ok(ProtocolResponse::NA),
        Err(e) => Err(e.into()),
    }
}
