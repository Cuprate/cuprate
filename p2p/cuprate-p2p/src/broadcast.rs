//! # Broadcast Router
//!
//! This module handles broadcasting messages to multiple peers with the [`BroadcastSvc`].

use bytes::Bytes;

use monero_p2p::ConnectionDirection;

/// A request to broadcast some data to all connected peers or a sub-set like all inbound or all outbound.
///
/// Only certain P2P messages are supported here: [`NewFluffyBlock`](monero_wire::protocol::NewFluffyBlock) and [`NewTransactions`](monero_wire::protocol::NewTransactions). These are the only
/// P2P messages that make sense to broadcast to multiple peers.
///
/// [`NewBlock`](monero_wire::protocol::NewBlock) has been excluded as monerod has had fluffy blocks for a while and
/// Cuprate sets fluffy blocks as a requirement during handshakes.
pub enum BroadcastRequest {
    /// Broadcast a block to the network. The block will be broadcast as a fluffy block to all peers.
    Block {
        /// The block.
        block_bytes: Bytes,
        /// The current chain height - will be 1 more than the blocks' height.
        current_blockchain_height: u64,
    },
    /// Broadcast transactions to the network. If a [`ConnectionDirection`] is set the transaction
    /// will only be broadcast to that sub-set of peers, if it is [`None`] then the transaction will
    /// be broadcast to all peers.
    Transactions(Vec<Bytes>, Option<ConnectionDirection>),
}
