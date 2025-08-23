use std::mem;

use tokio::{task::JoinSet, time::timeout};
use tower::{util::BoxCloneService, Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use cuprate_p2p_core::{
    client::InternalPeerID, handles::ConnectionHandle, NetworkZone, PeerRequest, PeerResponse,
    ProtocolRequest, ProtocolResponse,
};
use cuprate_wire::protocol::{ChainRequest, ChainResponse};

use crate::{
    block_downloader::{
        chain_tracker::{ChainEntry, ChainTracker},
        BlockDownloadError, ChainSvcRequest, ChainSvcResponse,
    },
    constants::{
        BLOCK_DOWNLOADER_REQUEST_TIMEOUT, INITIAL_CHAIN_REQUESTS_TO_SEND,
        MAX_BLOCKS_IDS_IN_CHAIN_ENTRY, MEDIUM_BAN,
    },
    peer_set::{ClientDropGuard, PeerSetRequest, PeerSetResponse},
};

/// Request a chain entry from a peer.
///
/// Because the block downloader only follows and downloads one chain we only have to send the block hash of
/// top block we have found and the genesis block, this is then called `short_history`.
pub(crate) async fn request_chain_entry_from_peer<N: NetworkZone>(
    mut client: ClientDropGuard<N>,
    short_history: [[u8; 32]; 2],
) -> Result<(ClientDropGuard<N>, ChainEntry<N>), BlockDownloadError> {
    let PeerResponse::Protocol(ProtocolResponse::GetChain(chain_res)) = client
        .ready()
        .await?
        .call(PeerRequest::Protocol(ProtocolRequest::GetChain(
            ChainRequest {
                block_ids: short_history.into(),
                prune: true,
            },
        )))
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
pub(super) async fn initial_chain_search<N: NetworkZone, C>(
    peer_set: &mut BoxCloneService<PeerSetRequest, PeerSetResponse<N>, tower::BoxError>,
    mut our_chain_svc: C,
) -> Result<ChainTracker<N>, BlockDownloadError>
where
    C: Service<ChainSvcRequest<N>, Response = ChainSvcResponse<N>, Error = tower::BoxError>,
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

    let PeerSetResponse::PeersWithMorePoW(clients) = peer_set
        .ready()
        .await?
        .call(PeerSetRequest::PeersWithMorePoW(cumulative_difficulty))
        .await?
    else {
        unreachable!();
    };
    let mut peers = clients.into_iter();

    let mut futs = JoinSet::new();

    let req = PeerRequest::Protocol(ProtocolRequest::GetChain(ChainRequest {
        block_ids: block_ids.into(),
        prune: false,
    }));

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
                let PeerResponse::Protocol(ProtocolResponse::GetChain(chain_res)) =
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
                    drop(mem::replace(res, task_res));
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
    let ChainSvcResponse::FindFirstUnknown(first_unknown_ret) = our_chain_svc
        .ready()
        .await?
        .call(ChainSvcRequest::FindFirstUnknown(hashes.clone()))
        .await?
    else {
        panic!("chain service sent wrong response.");
    };

    // We know all the blocks already
    // TODO: The peer could still be on a different chain, however the chain might just be too far split.
    let Some((first_unknown, expected_height)) = first_unknown_ret else {
        return Err(BlockDownloadError::FailedToFindAChainToFollow);
    };

    // The peer must send at least one block we already know.
    if first_unknown == 0 {
        peer_handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeerSentNoOverlappingBlocks);
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

    let tracker = ChainTracker::new(
        first_entry,
        expected_height,
        our_genesis,
        previous_id,
        &mut our_chain_svc,
    )
    .await
    .map_err(|_| BlockDownloadError::ChainInvalid)?;

    Ok(tracker)
}
