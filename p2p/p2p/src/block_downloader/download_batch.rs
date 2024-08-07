use std::collections::HashSet;

use monero_serai::{block::Block, transaction::Transaction};
use rayon::prelude::*;
use tokio::time::timeout;
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_fixed_bytes::ByteArrayVec;
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_p2p_core::{
    handles::ConnectionHandle, NetworkZone, PeerRequest, PeerResponse, ProtocolRequest,
    ProtocolResponse,
};
use cuprate_wire::protocol::{GetObjectsRequest, GetObjectsResponse};

use crate::{
    block_downloader::{BlockBatch, BlockDownloadError, BlockDownloadTaskResponse},
    client_pool::ClientPoolDropGuard,
    constants::{BLOCK_DOWNLOADER_REQUEST_TIMEOUT, MAX_TRANSACTION_BLOB_SIZE, MEDIUM_BAN},
};

/// Attempts to request a batch of blocks from a peer, returning [`BlockDownloadTaskResponse`].
#[instrument(
    level = "debug",
    name = "download_batch",
    skip_all,
    fields(
        start_height = expected_start_height,
        attempt = _attempt
    )
)]
pub async fn download_batch_task<N: NetworkZone>(
    client: ClientPoolDropGuard<N>,
    ids: ByteArrayVec<32>,
    previous_id: [u8; 32],
    expected_start_height: usize,
    _attempt: usize,
) -> BlockDownloadTaskResponse<N> {
    BlockDownloadTaskResponse {
        start_height: expected_start_height,
        result: request_batch_from_peer(client, ids, previous_id, expected_start_height).await,
    }
}

/// Requests a sequential batch of blocks from a peer.
///
/// This function will validate the blocks that were downloaded were the ones asked for and that they match
/// the expected height.
async fn request_batch_from_peer<N: NetworkZone>(
    mut client: ClientPoolDropGuard<N>,
    ids: ByteArrayVec<32>,
    previous_id: [u8; 32],
    expected_start_height: usize,
) -> Result<(ClientPoolDropGuard<N>, BlockBatch), BlockDownloadError> {
    let request = PeerRequest::Protocol(ProtocolRequest::GetObjects(GetObjectsRequest {
        blocks: ids.clone(),
        pruned: false,
    }));

    // Request the blocks and add a timeout to the request
    let blocks_response = timeout(BLOCK_DOWNLOADER_REQUEST_TIMEOUT, async {
        let PeerResponse::Protocol(ProtocolResponse::GetObjects(blocks_response)) =
            client.ready().await?.call(request).await?
        else {
            panic!("Connection task returned wrong response.");
        };

        Ok::<_, BlockDownloadError>(blocks_response)
    })
    .await
    .map_err(|_| BlockDownloadError::TimedOut)??;

    // Initial sanity checks
    if blocks_response.blocks.len() > ids.len() {
        client.info.handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeersResponseWasInvalid);
    }

    if blocks_response.blocks.len() != ids.len() {
        return Err(BlockDownloadError::PeerDidNotHaveRequestedData);
    }
    let peer_handle = client.info.handle.clone();

    let blocks = rayon_spawn_async(move || {
        deserialize_batch(
            blocks_response,
            expected_start_height,
            ids,
            previous_id,
            peer_handle,
        )
    })
    .await;

    let batch = blocks.inspect_err(|e| {
        // If the peers response was invalid, ban it.
        if matches!(e, BlockDownloadError::PeersResponseWasInvalid) {
            client.info.handle.ban_peer(MEDIUM_BAN);
        }
    })?;

    Ok((client, batch))
}

fn deserialize_batch(
    blocks_response: GetObjectsResponse,
    expected_start_height: usize,
    requested_ids: ByteArrayVec<32>,
    previous_id: [u8; 32],
    peer_handle: ConnectionHandle,
) -> Result<BlockBatch, BlockDownloadError> {
    let blocks = blocks_response
        .blocks
        .into_par_iter()
        .enumerate()
        .map(|(i, block_entry)| {
            let expected_height = i + expected_start_height;

            let mut size = block_entry.block.len();

            let block = Block::read(&mut block_entry.block.as_ref())
                .map_err(|_| BlockDownloadError::PeersResponseWasInvalid)?;

            let block_hash = block.hash();

            // Check the block matches the one requested and the peer sent enough transactions.
            if requested_ids[i] != block_hash || block.transactions.len() != block_entry.txs.len() {
                return Err(BlockDownloadError::PeersResponseWasInvalid);
            }

            // Check that the previous ID is correct for the first block.
            // This is to protect use against banning the wrong peer.
            // This must happen after the hash check.
            if i == 0 && block.header.previous != previous_id {
                tracing::warn!(
                    "Invalid chain, peer told us a block follows the chain when it doesn't."
                );

                // This peer probably did nothing wrong, it was the peer who told us this blockID which
                // is misbehaving.
                return Err(BlockDownloadError::ChainInvalid);
            }

            // Check the height lines up as expected.
            // This must happen after the hash check.
            if !block
                .number()
                .is_some_and(|height| height == expected_height)
            {
                tracing::warn!(
                    "Invalid chain, expected height: {expected_height}, got height: {:?}",
                    block.number()
                );

                // This peer probably did nothing wrong, it was the peer who told us this blockID which
                // is misbehaving.
                return Err(BlockDownloadError::ChainInvalid);
            }

            // Deserialize the transactions.
            let txs = block_entry
                .txs
                .take_normal()
                .ok_or(BlockDownloadError::PeersResponseWasInvalid)?
                .into_iter()
                .map(|tx_blob| {
                    size += tx_blob.len();

                    if tx_blob.len() > MAX_TRANSACTION_BLOB_SIZE {
                        return Err(BlockDownloadError::PeersResponseWasInvalid);
                    }

                    Transaction::read(&mut tx_blob.as_ref())
                        .map_err(|_| BlockDownloadError::PeersResponseWasInvalid)
                })
                .collect::<Result<Vec<_>, _>>()?;

            // Make sure the transactions in the block were the ones the peer sent.
            let mut expected_txs = block.transactions.iter().collect::<HashSet<_>>();

            for tx in &txs {
                if !expected_txs.remove(&tx.hash()) {
                    return Err(BlockDownloadError::PeersResponseWasInvalid);
                }
            }

            if !expected_txs.is_empty() {
                return Err(BlockDownloadError::PeersResponseWasInvalid);
            }

            Ok(((block, txs), size))
        })
        .collect::<Result<(Vec<_>, Vec<_>), _>>()?;

    Ok(BlockBatch {
        blocks: blocks.0,
        size: blocks.1.into_iter().sum(),
        peer_handle,
    })
}
