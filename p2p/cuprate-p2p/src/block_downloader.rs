use futures::future::BoxFuture;
use futures::stream::FuturesOrdered;
use futures::FutureExt;
use std::collections::HashSet;
use std::fmt::Debug;
use std::ops::Index;
use std::task::ready;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use monero_serai::{block::Block, transaction::Transaction};
use rayon::prelude::*;
use tower::{Service, ServiceExt};

use cuprate_helper::asynch::rayon_spawn_async;
use fixed_bytes::ByteArrayVec;
use monero_p2p::services::{PeerSyncRequest, PeerSyncResponse};
use monero_p2p::{handles::ConnectionHandle, NetworkZone, PeerRequest, PeerResponse, PeerSyncSvc};
use monero_wire::protocol::{ChainRequest, GetObjectsResponse};

use crate::constants::{MEDIUM_BAN, SHORT_BAN};
use crate::peer_set::PeerSet;
use crate::peer_set::{PeerSetRequest, PeerSetResponse};

#[derive(Debug)]
pub enum BlockDownloaderError {
    BlockInvalid,
    PeerGaveInvalidInfo,
    PeerDoesNotHaveData,
    InternalSvc(tower::BoxError),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Where {
    NotFound,
    MainChain(u64),
    AltChain(u64),
    Invalid,
}

pub trait Blockchain {
    fn chain_history(
        &mut self,
        from: Option<[u8; 32]>,
    ) -> impl Future<Output = Vec<[u8; 32]>> + Send;

    fn have_block(&mut self, block_id: [u8; 32]) -> impl Future<Output = Where> + Send;

    fn cumulative_difficulty(&mut self) -> impl Future<Output = u128> + Send;
}

pub struct NextChainEntry {
    next_ids: Vec<[u8; 32]>,
    parent: [u8; 32],
    start_height: u64,
}

pub struct BlockDownloader<N: NetworkZone, PSync, BC> {
    peer_sync_svc: PSync,
    peer_set: PeerSet<N>,

    next_chain_entry: NextChainEntry,

    returned_blocks: Vec<[u8; 32]>,
    our_chain: BC,

    incoming_blocks: FuturesOrdered<BlockDownloadFuture>,
}

impl<N: NetworkZone, PSync, BC> BlockDownloader<N, PSync, BC>
where
    PSync: PeerSyncSvc<N>,
    BC: Blockchain,
{
    pub async fn new(
        mut peer_sync_svc: PSync,
        mut peer_set: PeerSet<N>,
        mut our_chain: BC,
    ) -> Result<Option<Self>, BlockDownloaderError> {
        let Some(next_chain_entry) =
            get_next_chain_entry(&mut peer_sync_svc, &mut peer_set, &mut our_chain).await?
        else {
            return Ok(None);
        };

        tracing::info!(
            "Got next chain entry, number of blocks: {}, top block in entry: {}",
            next_chain_entry.next_ids.len(),
            hex::encode(next_chain_entry.next_ids.last().unwrap())
        );

        Ok(Some(BlockDownloader {
            peer_sync_svc,
            peer_set,
            next_chain_entry,
            returned_blocks: vec![],
            our_chain,
            incoming_blocks: FuturesOrdered::new(),
        }))
    }
}

async fn get_next_chain_entry<N: NetworkZone, PSync, BC>(
    peer_sync_svc: &mut PSync,
    peer_set: &mut PeerSet<N>,
    our_chain: &mut BC,
) -> Result<Option<NextChainEntry>, BlockDownloaderError>
where
    PSync: PeerSyncSvc<N>,
    BC: Blockchain,
{
    let our_history = our_chain.chain_history(None).await;

    let req = PeerRequest::GetChain(ChainRequest {
        block_ids: our_history.into(),
        prune: false,
    });

    let current_cumulative_difficulty = our_chain.cumulative_difficulty().await;

    tracing::info!(
        "Finding next chain entry from peers, current cumulative difficulty: {}.",
        current_cumulative_difficulty
    );

    loop {
        let PeerSyncResponse::PeersToSyncFrom(peers) = peer_sync_svc
            .ready()
            .await
            .map_err(BlockDownloaderError::InternalSvc)?
            .call(PeerSyncRequest::PeersToSyncFrom(
                current_cumulative_difficulty,
            ))
            .await
            .map_err(BlockDownloaderError::InternalSvc)?
        else {
            panic!("Peer sync service sent wrong response!");
        };

        if peers.is_empty() {
            tracing::info!("No peers found with a higher cumulative difficulty");
            return Ok(None);
        }

        let Ok(PeerSetResponse::PeerResponse(PeerResponse::GetChain(chain_res), con_handle)) =
            peer_set
                .ready()
                .await
                .map_err(BlockDownloaderError::InternalSvc)?
                .call(PeerSetRequest::LoadBalancedPeerSubSetRequest {
                    peers,
                    req: req.clone(),
                })
                .await
        else {
            continue;
        };

        if chain_res.cumulative_difficulty() <= current_cumulative_difficulty
            || chain_res.m_block_ids.is_empty()
        {
            tracing::debug!(
                "Peers cumulative difficulty dropped or start {}/ stop {} heights with amt of blocks {} incorrect. banning for {} seconds",
                chain_res.start_height,
                chain_res.total_height,
                chain_res.m_block_ids.len(),
                MEDIUM_BAN.as_secs()
            );

            con_handle.ban_peer(MEDIUM_BAN);
            continue;
        }

        let mut block_ids: Vec<[u8; 32]> = (&chain_res.m_block_ids).into();
        let start_height = chain_res.start_height;
        drop(chain_res);

        if !matches!(
            our_chain.have_block(block_ids[0]).await,
            Where::MainChain(_)
        ) {
            tracing::debug!(
                "First block did not overlap, banning peer for {} seconds.",
                MEDIUM_BAN.as_secs()
            );
            con_handle.ban_peer(MEDIUM_BAN);
            continue;
        }

        let Ok(new_idx) = find_new(our_chain, &block_ids, start_height.try_into().unwrap()).await
        else {
            tracing::debug!(
                "Error finding unknown hashes in chain entry return banning peer for {} seconds.",
                MEDIUM_BAN.as_secs()
            );

            con_handle.ban_peer(MEDIUM_BAN);
            continue;
        };

        let parent = block_ids[new_idx - 1];
        block_ids.drain(0..new_idx);

        return Ok(Some(NextChainEntry {
            next_ids: block_ids,
            parent,
            start_height,
        }));
    }
}

/// Does a binary search on the incoming block hashes to find the index of the first hash we
/// don't know about.
///
/// Will error if we encounter a hash of a block that we have marked as invalid.
async fn find_new<BC: Blockchain>(
    blockchain: &mut BC,
    incoming_chain: &[[u8; 32]],
    start_height: usize,
) -> Result<usize, BlockDownloaderError>
where
    BC: Blockchain,
{
    let mut size = incoming_chain.len();
    let mut left = 0;
    let mut right = size;

    while left < right {
        let mid = left + size / 2;

        let have_block = blockchain.have_block(incoming_chain[mid]).await;

        match have_block {
            Where::Invalid => return Err(BlockDownloaderError::BlockInvalid),
            Where::AltChain(height) | Where::MainChain(height) => {
                if height != u64::try_from(start_height + mid).unwrap() {
                    return Err(BlockDownloaderError::PeerGaveInvalidInfo);
                }

                left = mid + 1;
            }
            Where::NotFound => {
                right = mid;
            }
        }

        size = right - left;
    }

    Ok(left)
}

enum BlockDownloadState {
    GettingBlocks(
        BoxFuture<'static, Result<(GetObjectsResponse, ConnectionHandle), tower::BoxError>>,
    ),
    DeserializingBlocks(
        BoxFuture<'static, Result<Vec<(Block, Vec<Transaction>)>, BlockDownloaderError>>,
    ),
}

struct BlockDownloadFuture {
    blocks: ByteArrayVec<32>,
    state: BlockDownloadState,
}

impl Future for BlockDownloadFuture {
    type Output = Result<Vec<(Block, Vec<Transaction>)>, (BlockDownloaderError, ByteArrayVec<32>)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let expected_hashes = self.blocks.clone();

            match self.state {
                BlockDownloadState::GettingBlocks(ref mut blocks_fut) => {
                    let Ok((ret, con_handle)) = ready!(blocks_fut.poll_unpin(cx)) else {
                        return Poll::Ready(Err((
                            BlockDownloaderError::PeerDoesNotHaveData,
                            expected_hashes,
                        )));
                    };

                    if ret.blocks.is_empty() {
                        return Poll::Ready(Err((
                            BlockDownloaderError::PeerDoesNotHaveData,
                            self.blocks.clone(),
                        )));
                    }

                    if ret.blocks.len() > self.blocks.len() {
                        con_handle.ban_peer(MEDIUM_BAN);
                        return Poll::Ready(Err((
                            BlockDownloaderError::PeerGaveInvalidInfo,
                            expected_hashes,
                        )));
                    }

                    let deserialize_fut = rayon_spawn_async(|| {
                        handle_incoming_blocks(ret, expected_hashes, con_handle)
                    })
                    .boxed();

                    self.state = BlockDownloadState::DeserializingBlocks(deserialize_fut);
                }
                BlockDownloadState::DeserializingBlocks(ref mut deserialize_fut) => {
                    return deserialize_fut
                        .poll_unpin(cx)
                        .map_err(|e| (e, expected_hashes))
                }
            }
        }
    }
}

fn handle_incoming_blocks(
    block_entries: GetObjectsResponse,
    expected_blocks: ByteArrayVec<32>,
    con_handle: ConnectionHandle,
) -> Result<Vec<(Block, Vec<Transaction>)>, BlockDownloaderError> {
    block_entries
        .blocks
        .into_par_iter()
        .enumerate()
        .map(|(i, block_entry)| {
            let expected_hash = expected_blocks.index(i);

            let block = Block::read(&mut block_entry.block.as_ref()).map_err(|_| {
                con_handle.ban_peer(MEDIUM_BAN);
                BlockDownloaderError::PeerGaveInvalidInfo
            })?;

            if block.hash().as_slice() != expected_hash {
                // can't ban peer here as we could have been given an invalid list.
                return Err(BlockDownloaderError::PeerGaveInvalidInfo);
            }

            let mut expected_txs: HashSet<_> = block.txs.iter().collect();

            let txs_bytes = block_entry.txs.take_normal().unwrap_or_default();

            let txs = txs_bytes
                .into_iter()
                .map(|bytes| {
                    let tx = Transaction::read(&mut bytes.as_ref()).map_err(|_| {
                        con_handle.ban_peer(MEDIUM_BAN);
                        BlockDownloaderError::PeerGaveInvalidInfo
                    })?;

                    expected_txs.remove(&tx.hash());

                    Ok(tx)
                })
                .collect::<Result<Vec<_>, _>>()?;

            if !expected_txs.is_empty() {
                con_handle.ban_peer(SHORT_BAN);
                return Err(BlockDownloaderError::PeerGaveInvalidInfo);
            }

            Ok((block, txs))
        })
        .collect::<Result<Vec<_>, _>>()
}
