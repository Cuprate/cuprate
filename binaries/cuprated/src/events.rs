//! Node events
//!
//! Contains the [`NodeEvent`]s emitted by a running node.

use std::sync::{Arc, OnceLock};

use tokio::sync::broadcast;

/// The number of [`NodeEvent`]s a subscriber can fall behind before it starts to miss events.
const EVENT_CHANNEL_SIZE: usize = 1024;

/// An event emitted by a running `cuprated` node, received from [`Node::events`](crate::Node::events).
///
/// Events are sent after the change has been written to the database. A subscriber that falls too far behind
/// will miss events, the receiver returns [`RecvError::Lagged`](tokio::sync::broadcast::error::RecvError::Lagged)
/// when this happens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeEvent {
    /// Blocks were added to the main-chain.
    BlocksAdded {
        /// The height of the first block.
        first_height: usize,
        /// The previous block hash of the first block.
        first_prev_hash: [u8; 32],
        /// The new chain height - will be 1 more than the last block's height.
        chain_height: usize,
        /// The hash of the last block.
        top_hash: [u8; 32],
    },
    /// Blocks were popped from the top of the main-chain.
    BlocksPopped {
        /// The new chain height.
        chain_height: usize,
        /// The hash of the new top block.
        top_hash: [u8; 32],
    },
    /// A tx was added to the public tx-pool.
    TxAdded {
        /// The tx hash.
        hash: [u8; 32],
        /// The size of the tx blob.
        blob_size: usize,
        /// The weight of the tx.
        weight: usize,
        /// The fee the tx paid.
        fee: u64,
    },
    /// A tx was removed from the public tx-pool.
    TxRemoved {
        /// The tx hash.
        hash: [u8; 32],
        /// The reason the tx was removed.
        reason: TxRemovedReason,
    },
    /// The node has synchronised with the network, this is only sent once.
    Synced,
}

/// The reason a tx was removed from the public tx-pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TxRemovedReason {
    /// The tx was included in a block, or it double spent a tx that was.
    Block,
    /// The tx was in the tx-pool for too long.
    Expired,
}

/// The sender of [`NodeEvent`]s.
///
/// The channel is only created on the first [`EventSender::subscribe`], until then sending does nothing.
#[derive(Clone, Default)]
pub(crate) struct EventSender(Arc<OnceLock<broadcast::Sender<NodeEvent>>>);

impl EventSender {
    /// Sends a [`NodeEvent`] to all subscribers.
    pub(crate) fn send(&self, event: NodeEvent) {
        if let Some(tx) = self.0.get() {
            drop(tx.send(event));
        }
    }

    /// Returns a receiver for the [`NodeEvent`]s sent after this call.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<NodeEvent> {
        self.0
            .get_or_init(|| broadcast::channel(EVENT_CHANNEL_SIZE).0)
            .subscribe()
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::broadcast::error::TryRecvError;

    use super::*;

    #[test]
    fn event_sender() {
        let events = EventSender::default();
        // The node subscribes on one clone and emits on another.
        let emitter = events.clone();
        let emit = move || emitter.send(NodeEvent::Synced);

        emit();
        // Nothing has subscribed, the channel must not exist yet.
        assert!(events.0.get().is_none());

        let mut rx = events.subscribe();
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

        emit();
        assert_eq!(rx.try_recv(), Ok(NodeEvent::Synced));

        for _ in 0..=EVENT_CHANNEL_SIZE {
            emit();
        }
        assert_eq!(rx.try_recv(), Err(TryRecvError::Lagged(1)));

        // Sending with every subscriber gone is not an error.
        drop(rx);
        emit();
    }
}
