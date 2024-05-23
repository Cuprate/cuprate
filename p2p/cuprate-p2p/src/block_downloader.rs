//! # Block Downloader
//!

use std::sync::Arc;

use monero_serai::{block::Block, transaction::Transaction};

use async_buffer::BufferStream;
use monero_p2p::{handles::ConnectionHandle, NetworkZone};

use crate::client_pool::ClientPool;

/// A downloaded batch of blocks.
pub struct BlockBatch {
    /// The blocks.
    blocks: (Block, Vec<Transaction>),
    /// The peer that gave us this block.
    peer_handle: ConnectionHandle,
}

/// The block downloader config.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq)]
pub struct BlockDownloaderConfig {
    /// The size of the buffer between the block downloader and the place which
    /// is consuming the downloaded blocks.
    buffer_size: usize,
    /// The size of the in progress queue at which we stop requesting more blocks.
    in_progress_queue_size: usize,
}

/// # Block Downloader
///
/// This function starts the block downloader and returns a [`BufferStream`] that will produce
/// a sequential stream of blocks.
///
/// The block downloader will pick the longest chain and will follow it for as long as possible,
/// the blocks given from the [`BufferStream`] will be in order.
///
/// The block downloader may fail before the whole chain is downloaded. If this is the case you are
/// free to call this function again, so it can start the search again.
pub fn download_blocks<N: NetworkZone, S>(
    client_pool: Arc<ClientPool<N>>,
    peer_sync_svc: S,
    config: BlockDownloaderConfig,
) -> BufferStream<BlockBatch> {
    todo!()
}
