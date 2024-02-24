//! # Broadcast Router
//!
//! This module handles broadcasting messages to multiple peers with the [`BroadcastSvc`].
use std::marker::PhantomData;
use std::{
    convert::Infallible,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::future::{ready, Ready};
use tokio::sync::broadcast;
use tower::Service;

use monero_p2p::{protocol::PeerBroadcast, ConnectionDirection, NetworkZone};
use monero_wire::{
    common::{BlockCompleteEntry, TransactionBlobs},
    protocol::{NewFluffyBlock, NewTransactions},
};

/// A request to broadcast some data to all connected peers or a sub-set like all inbound or all outbound.
///
/// Only certain P2P messages are supported here: [`NewFluffyBlock`] and [`NewTransactions`]. These are the only
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

/// The broadcast service/ router.
///
/// This service handles broadcasting to peers, see [`BroadcastRequest`].
pub struct BroadcastSvc<N: NetworkZone> {
    /// A channel to broadcast messages to all outbound peers.
    outbound_broadcast: broadcast::Sender<PeerBroadcast>,
    /// A channel to broadcast messages to all inbound peers.
    inbound_broadcast: broadcast::Sender<PeerBroadcast>,
    /// A binding to the network. Different [`NetworkZone`] may broadcast in slightly different ways e.g.
    /// they may add padding to tx messages.
    _net: PhantomData<N>,
}

impl<N: NetworkZone> BroadcastSvc<N> {
    /// Broadcast a fluffy block to multiple peers: [`BroadcastRequest::Block`]
    pub fn broadcast_fluffy_block(&self, block: NewFluffyBlock) {
        let message = PeerBroadcast::NewFluffyBlock(block);

        let mut recivers = 0;
        match self.outbound_broadcast.send(message.clone()) {
            Ok(peers) => recivers += peers,
            Err(_) => tracing::info!("No outbound connections to broadcast new block to!"),
        };

        match self.inbound_broadcast.send(message.clone()) {
            Ok(peers) => recivers += peers,
            // A lot of nodes won't have ports open so this is only a debug log to prevent log spam.
            Err(_) => tracing::debug!("No inbound connection to broadcast new block to!"),
        };

        tracing::debug!("Attempting to broadcast new block to {} peers.", recivers);
    }

    /// Broadcast transactions to multiple peers: [`BroadcastRequest::Transactions`]
    pub fn broadcast_transactions(
        &self,
        txs: NewTransactions,
        direction: Option<ConnectionDirection>,
    ) {
        let message = PeerBroadcast::Transactions(txs);

        let (inbound, outbound) = match direction {
            None => (true, true),
            Some(ConnectionDirection::InBound) => (true, false),
            Some(ConnectionDirection::OutBound) => (false, true),
        };

        if outbound {
            match self.outbound_broadcast.send(message.clone()) {
                Ok(peers) => {
                    tracing::debug!("Attempting to broadcast txs to {} outbound peers", peers)
                }
                Err(_) => tracing::debug!("No outbound connections to broadcast new txs to!"),
            };
        }

        if inbound {
            match self.inbound_broadcast.send(message.clone()) {
                Ok(peers) => {
                    tracing::debug!("Attempting to broadcast txs to {} inbound peers", peers)
                }
                Err(_) => tracing::debug!("No inbound connections to broadcast new txs to!"),
            };
        }
    }
}

impl<N: NetworkZone> Service<BroadcastRequest> for BroadcastSvc<N> {
    type Response = ();
    type Error = Infallible;
    type Future = Ready<Result<(), Infallible>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: BroadcastRequest) -> Self::Future {
        match req {
            BroadcastRequest::Block {
                block_bytes,
                current_blockchain_height,
            } => {
                let fluffy_block = NewFluffyBlock {
                    b: BlockCompleteEntry {
                        pruned: false,
                        block: block_bytes,
                        block_weight: 0,             // Only needed for pruned blocks.
                        txs: TransactionBlobs::None, // This is a fluffy block - no txs.
                    },
                    current_blockchain_height,
                };

                self.broadcast_fluffy_block(fluffy_block);
            }
            BroadcastRequest::Transactions(txs, direction) => {
                let new_txs = NewTransactions {
                    txs,
                    dandelionpp_fluff: true, // We are broadcasting to multiple peers - aka fluff.
                    // TODO: we should use padding for anonymity networks.
                    padding: Bytes::new(),
                };

                self.broadcast_transactions(new_txs, direction)
            }
        }

        ready(Ok(()))
    }
}
