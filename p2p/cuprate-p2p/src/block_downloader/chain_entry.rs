use tokio::time::timeout;
use tower::{Service, ServiceExt};
use tracing::instrument;

use monero_p2p::{
    services::{PeerSyncRequest, PeerSyncResponse},
    NetworkZone, PeerRequest, PeerResponse, PeerSyncSvc,
};
use monero_wire::protocol::ChainRequest;

use crate::{
    block_downloader::{BlockDownloaderError, Blockchain, NextChainEntry, Where},
    constants::{CHAIN_REQUEST_TIMEOUT, MEDIUM_BAN},
    peer_set::{PeerSet, PeerSetRequest, PeerSetResponse},
};

#[instrument(level = "info", skip_all)]
pub(super) async fn get_next_chain_entry<N: NetworkZone, PSync, BC>(
    peer_sync_svc: &mut PSync,
    peer_set: &mut PeerSet<N>,
    our_chain: &mut BC,
    top_extra_block: Option<[u8; 32]>,
) -> Result<Option<NextChainEntry<N>>, BlockDownloaderError>
where
    PSync: PeerSyncSvc<N>,
    BC: Blockchain,
{
    let mut our_history = our_chain.chain_history(None).await;

    if let Some(top_extra_block) = top_extra_block {
        if our_history.len() == 1 && top_extra_block != our_history[0] {
            // we must keep the genesis block.
            our_history.insert(0, top_extra_block);
        } else {
            // if the genesis block is not the first we can just replace it.
            our_history[0] = top_extra_block
        }
    }

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

        let Ok(Ok(PeerSetResponse::PeerResponse(
            PeerResponse::GetChain(chain_res),
            peer_id,
            con_handle,
        ))) = timeout(
            CHAIN_REQUEST_TIMEOUT,
            peer_set
                .ready()
                .await
                .map_err(BlockDownloaderError::InternalSvc)?
                .call(PeerSetRequest::LoadBalancedPeerSubSetRequest {
                    peers,
                    req: req.clone(),
                }),
        )
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

        if top_extra_block.is_none()
            && !matches!(
                our_chain.have_block(block_ids[0]).await,
                Where::MainChain(_)
            )
        {
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

        block_ids.drain(0..new_idx);

        return Ok(Some(NextChainEntry {
            next_ids: block_ids,
            peer: peer_id,
            handle: con_handle,
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
