use std::{
    collections::HashSet,
    future::Future,
    ops::Index,
    pin::Pin,
    task::{ready, Context, Poll},
};

use futures::{future::BoxFuture, FutureExt};
use monero_serai::{block::Block, transaction::Transaction};
use pin_project::pin_project;
use rayon::prelude::*;
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

use cuprate_helper::asynch::rayon_spawn_async;
use fixed_bytes::ByteArrayVec;
use monero_p2p::{handles::ConnectionHandle, NetworkZone, PeerRequest, PeerResponse};
use monero_wire::protocol::GetObjectsResponse;

use crate::{
    block_downloader::BlockDownloaderError,
    constants::{MEDIUM_BAN, SHORT_BAN},
    peer_set::PeerSetResponse,
};

/// The output of the [`BlockDownloadFuture`].
pub struct DownloadedBlocks {
    /// The downloaded blocks.
    pub blocks: Vec<(Block, Vec<Transaction>)>,
    /// The total size of the serialised blocks/txs.
    pub size: usize,
    /// An ID for this request.
    pub request_id: u64,
}

/// Block downloading state.
enum BlockDownloadState<N: NetworkZone> {
    /// Waiting for the blocks from the peer.
    GettingBlocks(BoxFuture<'static, Result<PeerSetResponse<N>, tower::BoxError>>),
    /// Waiting for the blocks to be deserialized.
    DeserializingBlocks(BoxFuture<'static, Result<DownloadedBlocks, BlockDownloaderError>>),
}

/// A [`Future`] that completes when a peer responds to a block request and the blocks have been deserialized.
#[pin_project::pin_project]
pub(super) struct BlockDownloadFuture<N: NetworkZone> {
    /// The block hashes we have requested.
    blocks: ByteArrayVec<32>,
    /// The current state.
    state: BlockDownloadState<N>,
    /// A bool to mark if we should ban this peer if it does not have the requested blocks.
    drop_peer_if_not_found: bool,
    /// An ID for this request.
    request_id: u64,

    #[pin]
    cancel_token: WaitForCancellationFutureOwned,
}

impl<N: NetworkZone> BlockDownloadFuture<N> {
    /// Creates a new [`BlockDownloadFuture`].
    ///
    /// The inputted `req_fut` must have been a [`PeerRequest::GetObjects`] otherwise this will panic.
    pub(super) fn new(
        blocks: ByteArrayVec<32>,
        req_fut: BoxFuture<'static, Result<PeerSetResponse<N>, tower::BoxError>>,
        drop_peer_if_not_found: bool,
        request_id: u64,
        cancel_token: CancellationToken,
    ) -> BlockDownloadFuture<N> {
        BlockDownloadFuture {
            blocks,
            state: BlockDownloadState::GettingBlocks(req_fut),
            drop_peer_if_not_found,
            request_id,
            cancel_token: cancel_token.cancelled_owned(),
        }
    }
}

impl<N: NetworkZone> Future for BlockDownloadFuture<N> {
    type Output = Result<DownloadedBlocks, (BlockDownloaderError, u64)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        loop {
            if this.cancel_token.as_mut().poll(cx).is_ready() {
                return Poll::Ready(Err((
                    BlockDownloaderError::PeerDoesNotHaveData,
                    *this.request_id,
                )));
            }

            match this.state {
                BlockDownloadState::GettingBlocks(ref mut blocks_fut) => {
                    let Ok(res) = ready!(blocks_fut.poll_unpin(cx)) else {
                        return Poll::Ready(Err((
                            BlockDownloaderError::PeerDoesNotHaveData,
                            *this.request_id,
                        )));
                    };

                    let PeerSetResponse::PeerResponse(
                        PeerResponse::GetObjects(ret),
                        _id,
                        con_handle,
                    ) = res
                    else {
                        panic!("Peer set/ connection tasked returned incorrect response.")
                    };

                    // This will do a shallow clone.
                    let expected_hashes = this.blocks.clone();

                    if ret.blocks.len() < expected_hashes.len() {
                        tracing::debug!("Peer did not send all the blocks we requested.");

                        if *this.drop_peer_if_not_found {
                            con_handle.ban_peer(MEDIUM_BAN);
                        }

                        return Poll::Ready(Err((
                            BlockDownloaderError::PeerDoesNotHaveData,
                            *this.request_id,
                        )));
                    }

                    if ret.blocks.len() > expected_hashes.len() {
                        tracing::debug!("Peer sent more blocks than we requested.");

                        // if the peer responded with more blocks than we asked for then ban it.
                        con_handle.ban_peer(MEDIUM_BAN);

                        return Poll::Ready(Err((
                            BlockDownloaderError::PeerGaveInvalidInfo,
                            *this.request_id,
                        )));
                    }

                    tracing::debug!(
                        "Received {} blocks for request_id {}, deserializing.",
                        ret.blocks.len(),
                        this.request_id
                    );

                    let request_id = *this.request_id;
                    let deserialize_fut = rayon_spawn_async(move || {
                        deserialize_incoming_blocks(ret, expected_hashes, con_handle, request_id)
                    })
                    .boxed();

                    *this.state = BlockDownloadState::DeserializingBlocks(deserialize_fut);
                }
                BlockDownloadState::DeserializingBlocks(ref mut deserialize_fut) => {
                    return deserialize_fut
                        .poll_unpin(cx)
                        .map_err(|e| (e, *this.request_id))
                }
            }
        }
    }
}

/// Deserializes the incoming blocks and checks that they are the ones we asked for.
fn deserialize_incoming_blocks(
    block_entries: GetObjectsResponse,
    expected_blocks: ByteArrayVec<32>,
    con_handle: ConnectionHandle,
    request_id: u64,
) -> Result<DownloadedBlocks, BlockDownloaderError> {
    // TODO: do size checks before deserializing

    let (blocks, sizes) = block_entries
        .blocks
        .into_par_iter()
        .enumerate()
        .map(|(i, block_entry)| {
            let mut size = block_entry.block.len();

            let expected_hash = expected_blocks.index(i);

            let block = Block::read(&mut block_entry.block.as_ref()).map_err(|_| {
                tracing::debug!("Peer sent block we can't deserialize, banning.");
                con_handle.ban_peer(MEDIUM_BAN);
                BlockDownloaderError::PeerGaveInvalidInfo
            })?;

            if block.hash().as_slice() != expected_hash {
                tracing::debug!("Peer sent block we did not ask for, banning.");

                // We can ban here because we check if we are given the amount of blocks we asked for.
                con_handle.ban_peer(MEDIUM_BAN);
                return Err(BlockDownloaderError::PeerGaveInvalidInfo);
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
                        BlockDownloaderError::PeerGaveInvalidInfo
                    })?;

                    expected_txs.remove(&tx.hash());

                    Ok(tx)
                })
                .collect::<Result<Vec<_>, _>>()?;

            if !expected_txs.is_empty() {
                tracing::debug!("Peer did not send all transactions in the block, banning.");

                con_handle.ban_peer(SHORT_BAN);
                return Err(BlockDownloaderError::PeerGaveInvalidInfo);
            }

            Ok(((block, txs), size))
        })
        .collect::<Result<(Vec<_>, Vec<_>), _>>()?;

    Ok(DownloadedBlocks {
        blocks,
        size: sizes.into_iter().sum(),
        request_id,
    })
}
