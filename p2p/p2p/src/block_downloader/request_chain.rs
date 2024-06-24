use std::{mem, sync::Arc};

use rand::prelude::SliceRandom;
use rand::thread_rng;
use tokio::{task::JoinSet, time::timeout};
use tower::{Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use cuprate_p2p_core::{
    client::InternalPeerID,
    handles::ConnectionHandle,
    services::{PeerSyncRequest, PeerSyncResponse},
    NetworkZone, PeerRequest, PeerResponse, PeerSyncSvc,
};
use cuprate_wire::protocol::{ChainRequest, ChainResponse};

use crate::{
    block_downloader::{
        chain_tracker::{ChainEntry, ChainTracker},
        BlockDownloadError, ChainSvcRequest, ChainSvcResponse,
    },
    client_pool::{ClientPool, ClientPoolDropGuard},
    constants::{
        BLOCK_DOWNLOADER_REQUEST_TIMEOUT, INITIAL_CHAIN_REQUESTS_TO_SEND,
        MAX_BLOCKS_IDS_IN_CHAIN_ENTRY, MEDIUM_BAN,
    },
};

/// Request a chain entry from a peer.
///
/// Because the block downloader only follows and downloads one chain we only have to send the block hash of
/// top block we have found and the genesis block, this is then called `short_history`.
pub async fn request_chain_entry_from_peer<N: NetworkZone>(
    mut client: ClientPoolDropGuard<N>,
    short_history: [[u8; 32]; 2],
) -> Result<(ClientPoolDropGuard<N>, ChainEntry<N>), BlockDownloadError> {
    let PeerResponse::GetChain(chain_res) = client
        .ready()
        .await?
        .call(PeerRequest::GetChain(ChainRequest {
            block_ids: short_history.into(),
            prune: true,
        }))
        .await?
    else {
        panic!("Connection task returned wrong response!");
    };

    if chain_res.m_block_ids.is_empty()
        || chain_res.m_block_ids.len() > MAX_BLOCKS_IDS_IN_CHAIN_ENTRY
    {
        client.info.handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeersResponseWasInvalid);
    }

    // We must have at least one overlapping block.
    if !(chain_res.m_block_ids[0] == short_history[0]
        || chain_res.m_block_ids[0] == short_history[1])
    {
        client.info.handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeersResponseWasInvalid);
    }

    // If the genesis is the overlapping block then this peer does not have our top tracked block in
    // its chain.
    if chain_res.m_block_ids[0] == short_history[1] {
        return Err(BlockDownloadError::PeerDidNotHaveRequestedData);
    }

    let entry = ChainEntry {
        ids: (&chain_res.m_block_ids).into(),
        peer: client.info.id,
        handle: client.info.handle.clone(),
    };

    Ok((client, entry))
}

/// Initial chain search, this function pulls [`INITIAL_CHAIN_REQUESTS_TO_SEND`] peers from the [`ClientPool`]
/// and sends chain requests to all of them.
///
/// We then wait for their response and choose the peer who claims the highest cumulative difficulty.
#[instrument(level = "error", skip_all)]
pub async fn initial_chain_search<N: NetworkZone, S, C>(
    client_pool: &Arc<ClientPool<N>>,
    mut peer_sync_svc: S,
    mut our_chain_svc: C,
) -> Result<ChainTracker<N>, BlockDownloadError>
where
    S: PeerSyncSvc<N>,
    C: Service<ChainSvcRequest, Response = ChainSvcResponse, Error = tower::BoxError>,
{
    tracing::debug!("Getting our chain history");
    // Get our history.
    let ChainSvcResponse::CompactHistory {
        block_ids,
        cumulative_difficulty,
    } = our_chain_svc
        .ready()
        .await?
        .call(ChainSvcRequest::CompactHistory)
        .await?
    else {
        panic!("chain service sent wrong response.");
    };

    let our_genesis = *block_ids.last().expect("Blockchain had no genesis block.");

    tracing::debug!("Getting a list of peers with higher cumulative difficulty");

    let PeerSyncResponse::PeersToSyncFrom(mut peers) = peer_sync_svc
        .ready()
        .await?
        .call(PeerSyncRequest::PeersToSyncFrom {
            block_needed: None,
            current_cumulative_difficulty: cumulative_difficulty,
        })
        .await?
    else {
        panic!("peer sync service sent wrong response.");
    };

    tracing::debug!(
        "{} peers claim they have a higher cumulative difficulty",
        peers.len()
    );

    // Shuffle the list to remove any possibility of peers being able to prioritize getting picked.
    peers.shuffle(&mut thread_rng());

    let mut peers = client_pool.borrow_clients(&peers);

    let mut futs = JoinSet::new();

    let req = PeerRequest::GetChain(ChainRequest {
        block_ids: block_ids.into(),
        prune: false,
    });

    tracing::debug!("Sending requests for chain entries.");

    // Send the requests.
    while futs.len() < INITIAL_CHAIN_REQUESTS_TO_SEND {
        let Some(mut next_peer) = peers.next() else {
            break;
        };

        let cloned_req = req.clone();
        futs.spawn(timeout(
            BLOCK_DOWNLOADER_REQUEST_TIMEOUT,
            async move {
                let PeerResponse::GetChain(chain_res) =
                    next_peer.ready().await?.call(cloned_req).await?
                else {
                    panic!("connection task returned wrong response!");
                };

                Ok::<_, tower::BoxError>((
                    chain_res,
                    next_peer.info.id,
                    next_peer.info.handle.clone(),
                ))
            }
            .instrument(Span::current()),
        ));
    }

    let mut res: Option<(ChainResponse, InternalPeerID<_>, ConnectionHandle)> = None;

    // Wait for the peers responses.
    while let Some(task_res) = futs.join_next().await {
        let Ok(Ok(task_res)) = task_res.unwrap() else {
            continue;
        };

        match &mut res {
            Some(res) => {
                // res has already been set, replace it if this peer claims higher cumulative difficulty
                if res.0.cumulative_difficulty() < task_res.0.cumulative_difficulty() {
                    let _ = mem::replace(res, task_res);
                }
            }
            None => {
                // res has not been set, set it now;
                res = Some(task_res);
            }
        }
    }

    let Some((chain_res, peer_id, peer_handle)) = res else {
        return Err(BlockDownloadError::FailedToFindAChainToFollow);
    };

    let hashes: Vec<[u8; 32]> = (&chain_res.m_block_ids).into();
    // drop this to deallocate the [`Bytes`].
    drop(chain_res);

    tracing::debug!("Highest chin entry contained {} block Ids", hashes.len());

    // Find the first unknown block in the batch.
    let ChainSvcResponse::FindFirstUnknown(first_unknown, expected_height) = our_chain_svc
        .ready()
        .await?
        .call(ChainSvcRequest::FindFirstUnknown(hashes.clone()))
        .await?
    else {
        panic!("chain service sent wrong response.");
    };

    // The peer must send at least one block we already know.
    if first_unknown == 0 {
        peer_handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeerSentNoOverlappingBlocks);
    }

    // We know all the blocks already
    // TODO: The peer could still be on a different chain, however the chain might just be too far split.
    if first_unknown == hashes.len() {
        return Err(BlockDownloadError::FailedToFindAChainToFollow);
    }

    let previous_id = hashes[first_unknown - 1];

    let first_entry = ChainEntry {
        ids: hashes[first_unknown..].to_vec(),
        peer: peer_id,
        handle: peer_handle,
    };

    tracing::debug!(
        "Creating chain tracker with {} new block Ids",
        first_entry.ids.len()
    );

    let tracker = ChainTracker::new(first_entry, expected_height, our_genesis, previous_id);

    Ok(tracker)
}
