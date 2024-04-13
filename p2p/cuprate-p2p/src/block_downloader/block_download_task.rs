use monero_serai::block::Block;
use monero_serai::transaction::Transaction;
use rayon::prelude::*;
use std::collections::HashSet;
use std::ops::Index;
use tokio_util::sync::CancellationToken;
use tower::{Service, ServiceExt};
use tracing::instrument;

use crate::constants::{MEDIUM_BAN, SHORT_BAN};
use crate::peer_set::ClientPoolGuard;
use cuprate_helper::asynch::rayon_spawn_async;
use fixed_bytes::ByteArrayVec;
use monero_p2p::{NetworkZone, PeerRequest, PeerResponse};
use monero_wire::protocol::GetObjectsRequest;

#[derive(Debug)]
pub enum DownloadBlocksErrorInner {
    PeerSentIncorrectAmountOfBlocks,
    PeerGaveInvalidBlocks,
    Cancelled,
    PeerClientError(tower::BoxError),
}

pub struct DownloadedBlocks {
    pub blocks: Vec<(Block, Vec<Transaction>)>,
    pub size: usize,
}

pub struct GetBlocksOk<N: NetworkZone> {
    pub client: ClientPoolGuard<N>,
    pub request_id: u64,
    pub blocks: DownloadedBlocks,
}

pub struct DownloadBlocksError {
    pub request_id: u64,
    pub error: DownloadBlocksErrorInner,
}

#[instrument(level = "info", skip_all, fields(request_id=request_id, %client.info.id))]
pub async fn get_blocks<N: NetworkZone>(
    mut client: ClientPoolGuard<N>,
    block_ids: ByteArrayVec<32>,
    request_id: u64,
    cancellation_token: CancellationToken,
) -> Result<GetBlocksOk<N>, DownloadBlocksError> {
    tracing::debug!("Sending request for {} blocks", block_ids.len());

    let map_err = |e| DownloadBlocksError {
        request_id,
        error: DownloadBlocksErrorInner::PeerClientError(e),
    };

    let PeerResponse::GetObjects(block_res) = client
        .ready()
        .await
        .map_err(map_err)?
        .call(PeerRequest::GetObjects(GetObjectsRequest {
            blocks: block_ids.clone(),
            pruned: false,
        }))
        .await
        .map_err(map_err)?
    else {
        panic!("Connection task returned wrong response to request");
    };

    if block_res.blocks.len() != block_ids.len() || block_res.missed_ids.len() != 0 {
        tracing::warn!(
            "Peer responded with incorrect amount of blocks, requested: {}, got: {}",
            block_ids.len(),
            block_res.blocks.len()
        );

        if block_res.blocks.len() > block_ids.len() {
            client.info.handle.ban_peer(SHORT_BAN);
        }

        return Err(DownloadBlocksError {
            request_id,
            error: DownloadBlocksErrorInner::PeerSentIncorrectAmountOfBlocks,
        });
    }

    if cancellation_token.is_cancelled() {
        tracing::debug!("Request has been cancelled, not deserializing blocks.");

        return Err(DownloadBlocksError {
            request_id,
            error: DownloadBlocksErrorInner::Cancelled,
        });
    }

    let con_handle = client.info.handle.clone();
    let span = tracing::Span::current();

    let res = rayon_spawn_async(move || {
        let _guard = span.enter();

        let (blocks, sizes) = block_res
            .blocks
            .into_par_iter()
            .enumerate()
            .map(|(i, block_entry)| {
                let mut size = block_entry.block.len();

                let expected_hash = block_ids.index(i);
                // TODO: make sure each read reads all the bytes and add size limits.
                let block = Block::read(&mut block_entry.block.as_ref()).map_err(|_| {
                    tracing::debug!("Peer sent block we can't deserialize, banning.");
                    con_handle.ban_peer(MEDIUM_BAN);
                    DownloadBlocksErrorInner::PeerGaveInvalidBlocks
                })?;

                if block.hash().as_slice() != expected_hash {
                    tracing::debug!("Peer sent block we did not ask for, banning.");

                    // We can ban here because we check if we are given the amount of blocks we asked for.
                    con_handle.ban_peer(MEDIUM_BAN);
                    return Err(DownloadBlocksErrorInner::PeerGaveInvalidBlocks);
                }

                let mut expected_txs: HashSet<_> = block.txs.iter().collect();

                let txs_bytes = block_entry.txs.take_normal().unwrap_or_default();

                let txs = txs_bytes
                    .into_iter()
                    .map(|bytes| {
                        size += bytes.len();

                        let tx = Transaction::read(&mut bytes.as_ref()).map_err(|_| {
                            tracing::debug!("Peer sent transaction we can't deserialize, banning.");
                            con_handle.ban_peer(MEDIUM_BAN);
                            DownloadBlocksErrorInner::PeerGaveInvalidBlocks
                        })?;

                        expected_txs.remove(&tx.hash());

                        Ok(tx)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                if !expected_txs.is_empty() {
                    tracing::debug!("Peer did not send all transactions in the block, banning.");

                    con_handle.ban_peer(SHORT_BAN);
                    return Err(DownloadBlocksErrorInner::PeerGaveInvalidBlocks);
                }

                Ok(((block, txs), size))
            })
            .collect::<Result<(Vec<_>, Vec<_>), _>>()?;

        Result::<_, DownloadBlocksErrorInner>::Ok(GetBlocksOk {
            client,
            request_id,
            blocks: DownloadedBlocks {
                blocks,
                size: sizes.into_iter().sum(),
            },
        })
    })
    .await;

    res.map_err(|error| DownloadBlocksError { request_id, error })
}
