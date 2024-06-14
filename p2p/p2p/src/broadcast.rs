//! # Broadcast Router
//!
//! This module handles broadcasting messages to multiple peers with the [`BroadcastSvc`].
use std::{
    future::{ready, Future, Ready},
    pin::{pin, Pin},
    task::{ready, Context, Poll},
    time::Duration,
};

use bytes::Bytes;
use futures::Stream;
use rand::prelude::*;
use rand_distr::Exp;
use tokio::{
    sync::{
        broadcast::{self, error::TryRecvError},
        watch,
    },
    time::{sleep_until, Instant, Sleep},
};
use tokio_stream::wrappers::WatchStream;
use tower::Service;

use monero_p2p::{client::InternalPeerID, BroadcastMessage, ConnectionDirection, NetworkZone};
use monero_wire::{
    common::{BlockCompleteEntry, TransactionBlobs},
    protocol::{NewFluffyBlock, NewTransactions},
};

use crate::constants::{
    DIFFUSION_FLUSH_AVERAGE_SECONDS_INBOUND, DIFFUSION_FLUSH_AVERAGE_SECONDS_OUTBOUND,
    MAX_TXS_IN_BROADCAST_CHANNEL, SOFT_TX_MESSAGE_SIZE_SIZE_LIMIT,
};

/// The configuration for the [`BroadcastSvc`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BroadcastConfig {
    /// The average number of seconds between diffusion flushes for outbound connections.
    pub diffusion_flush_average_seconds_outbound: Duration,
    /// The average number of seconds between diffusion flushes for inbound connections.
    pub diffusion_flush_average_seconds_inbound: Duration,
}

impl Default for BroadcastConfig {
    fn default() -> Self {
        Self {
            diffusion_flush_average_seconds_inbound: DIFFUSION_FLUSH_AVERAGE_SECONDS_INBOUND,
            diffusion_flush_average_seconds_outbound: DIFFUSION_FLUSH_AVERAGE_SECONDS_OUTBOUND,
        }
    }
}

/// Initialise the [`BroadcastSvc`] and the functions to produce [`BroadcastMessageStream`]s.
///
/// This function will return in order:
/// - The [`BroadcastSvc`]
/// - A function that takes in [`InternalPeerID`]s and produces [`BroadcastMessageStream`]s to give to **outbound** peers.
/// - A function that takes in [`InternalPeerID`]s and produces [`BroadcastMessageStream`]s to give to **inbound** peers.
pub fn init_broadcast_channels<N: NetworkZone>(
    config: BroadcastConfig,
) -> (
    BroadcastSvc<N>,
    impl Fn(InternalPeerID<N::Addr>) -> BroadcastMessageStream<N> + Clone + Send + 'static,
    impl Fn(InternalPeerID<N::Addr>) -> BroadcastMessageStream<N> + Clone + Send + 'static,
) {
    let outbound_dist = Exp::new(
        1.0 / config
            .diffusion_flush_average_seconds_outbound
            .as_secs_f64(),
    )
    .unwrap();
    let inbound_dist =
        Exp::new(1.0 / config.diffusion_flush_average_seconds_inbound.as_secs_f64()).unwrap();

    // Set a default value for init - the broadcast streams given to the peer tasks will only broadcast from this channel when the value
    // changes so no peer will get sent this.
    let (block_watch_sender, block_watch_receiver) = watch::channel(NewBlockInfo {
        block_bytes: Default::default(),
        current_blockchain_height: 0,
    });

    // create the inbound/outbound broadcast channels.
    let (tx_broadcast_channel_outbound_sender, tx_broadcast_channel_outbound_receiver) =
        broadcast::channel(MAX_TXS_IN_BROADCAST_CHANNEL);
    let (tx_broadcast_channel_inbound_sender, tx_broadcast_channel_inbound_receiver) =
        broadcast::channel(MAX_TXS_IN_BROADCAST_CHANNEL);

    // create the broadcast service.
    let broadcast_svc = BroadcastSvc {
        new_block_watch: block_watch_sender,
        tx_broadcast_channel_outbound: tx_broadcast_channel_outbound_sender,
        tx_broadcast_channel_inbound: tx_broadcast_channel_inbound_sender,
    };

    // wrap the tx broadcast channels in a wrapper that impls Clone so the closures later on impl clone.
    let tx_channel_outbound_receiver_wrapped =
        CloneableBroadcastReceiver(tx_broadcast_channel_outbound_receiver);
    let tx_channel_inbound_receiver_wrapped =
        CloneableBroadcastReceiver(tx_broadcast_channel_inbound_receiver);

    // Create the closures that will be used to start the broadcast streams that the connection task will hold to listen
    // for messages to broadcast.
    let block_watch_receiver_cloned = block_watch_receiver.clone();
    let outbound_stream_maker = move |addr| {
        BroadcastMessageStream::new(
            addr,
            outbound_dist,
            block_watch_receiver_cloned.clone(),
            tx_channel_outbound_receiver_wrapped.clone().0,
        )
    };

    let inbound_stream_maker = move |addr| {
        BroadcastMessageStream::new(
            addr,
            inbound_dist,
            block_watch_receiver.clone(),
            tx_channel_inbound_receiver_wrapped.clone().0,
        )
    };

    (broadcast_svc, outbound_stream_maker, inbound_stream_maker)
}

/// A request to broadcast some data to all connected peers or a sub-set like all inbound or all outbound.
///
/// Only certain P2P messages are supported here: [`NewFluffyBlock`] and [`NewTransactions`]. These are the only
/// P2P messages that make sense to broadcast to multiple peers.
///
/// [`NewBlock`](monero_wire::protocol::NewBlock) has been excluded as monerod has had fluffy blocks for a while and
/// Cuprate sets fluffy blocks as a requirement during handshakes.
pub enum BroadcastRequest<N: NetworkZone> {
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
    Transaction {
        /// The serialised tx to broadcast.
        tx_bytes: Bytes,
        /// The direction of peers to broadcast this tx to, if [`None`] it will be sent to all peers.
        direction: Option<ConnectionDirection>,
        /// The peer on this network that told us about the tx.
        received_from: Option<InternalPeerID<N::Addr>>,
    },
}

#[derive(Clone)]
pub struct BroadcastSvc<N: NetworkZone> {
    new_block_watch: watch::Sender<NewBlockInfo>,
    tx_broadcast_channel_outbound: broadcast::Sender<BroadcastTxInfo<N>>,
    tx_broadcast_channel_inbound: broadcast::Sender<BroadcastTxInfo<N>>,
}

impl<N: NetworkZone> Service<BroadcastRequest<N>> for BroadcastSvc<N> {
    type Response = ();
    type Error = std::convert::Infallible;
    type Future = Ready<Result<(), std::convert::Infallible>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: BroadcastRequest<N>) -> Self::Future {
        match req {
            BroadcastRequest::Block {
                block_bytes,
                current_blockchain_height,
            } => {
                tracing::debug!(
                    "queuing block at chain height {current_blockchain_height} for broadcast"
                );

                self.new_block_watch.send_replace(NewBlockInfo {
                    block_bytes,
                    current_blockchain_height,
                });
            }
            BroadcastRequest::Transaction {
                tx_bytes,
                received_from,
                direction,
            } => {
                let nex_tx_info = BroadcastTxInfo {
                    tx: tx_bytes,
                    received_from,
                };

                // An error here means _all_ receivers were dropped which we assume will never happen.
                let _ = match direction {
                    Some(ConnectionDirection::InBound) => {
                        self.tx_broadcast_channel_inbound.send(nex_tx_info)
                    }
                    Some(ConnectionDirection::OutBound) => {
                        self.tx_broadcast_channel_outbound.send(nex_tx_info)
                    }
                    None => {
                        let _ = self.tx_broadcast_channel_outbound.send(nex_tx_info.clone());
                        self.tx_broadcast_channel_inbound.send(nex_tx_info)
                    }
                };
            }
        }

        ready(Ok(()))
    }
}

/// A wrapper type that impls [`Clone`] for [`broadcast::Receiver`].
///
/// The clone impl just calls [`Receiver::resubscribe`](broadcast::Receiver::resubscribe), which isn't _exactly_
/// a clone but is what we need for our use case.
struct CloneableBroadcastReceiver<T: Clone>(broadcast::Receiver<T>);

impl<T: Clone> Clone for CloneableBroadcastReceiver<T> {
    fn clone(&self) -> Self {
        Self(self.0.resubscribe())
    }
}

/// A new block to broadcast.
#[derive(Clone)]
struct NewBlockInfo {
    /// The block.
    block_bytes: Bytes,
    /// The current chain height - will be 1 more than the blocks' height.
    current_blockchain_height: u64,
}

/// A new transaction to broadcast.
#[derive(Clone)]
struct BroadcastTxInfo<N: NetworkZone> {
    /// The tx.
    tx: Bytes,
    /// The peer that sent us this tx (if the peer is on this network).
    received_from: Option<InternalPeerID<N::Addr>>,
}

/// A [`Stream`] that returns [`BroadcastMessage`] to broadcast to a peer.
///
/// This is given to the connection task to await on for broadcast messages.
#[pin_project::pin_project]
pub struct BroadcastMessageStream<N: NetworkZone> {
    /// The peer that is holding this stream.
    addr: InternalPeerID<N::Addr>,

    /// The channel where new blocks are received.
    #[pin]
    new_block_watch: WatchStream<NewBlockInfo>,
    /// The channel where txs to broadcast are received.
    tx_broadcast_channel: broadcast::Receiver<BroadcastTxInfo<N>>,

    /// The distribution to generate the wait time before the next transaction
    /// diffusion flush.
    diffusion_flush_dist: Exp<f64>,
    /// A [`Sleep`] that will awake when it's time to broadcast txs.
    #[pin]
    next_flush: Sleep,
}

impl<N: NetworkZone> BroadcastMessageStream<N> {
    /// Creates a new [`BroadcastMessageStream`]
    fn new(
        addr: InternalPeerID<N::Addr>,
        diffusion_flush_dist: Exp<f64>,
        new_block_watch: watch::Receiver<NewBlockInfo>,
        tx_broadcast_channel: broadcast::Receiver<BroadcastTxInfo<N>>,
    ) -> Self {
        let next_flush = Instant::now()
            + Duration::from_secs_f64(diffusion_flush_dist.sample(&mut thread_rng()));

        Self {
            addr,
            // We don't want to broadcast the message currently in the queue.
            new_block_watch: WatchStream::from_changes(new_block_watch),
            tx_broadcast_channel,
            diffusion_flush_dist,
            next_flush: sleep_until(next_flush),
        }
    }
}

impl<N: NetworkZone> Stream for BroadcastMessageStream<N> {
    type Item = BroadcastMessage;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Prioritise blocks.
        if let Poll::Ready(res) = this.new_block_watch.poll_next(cx) {
            let Some(block) = res else {
                return Poll::Ready(None);
            };

            let block_mes = NewFluffyBlock {
                b: BlockCompleteEntry {
                    pruned: false,
                    block: block.block_bytes,
                    // This is a full fluffy block these values do not need to be set.
                    block_weight: 0,
                    txs: TransactionBlobs::None,
                },
                current_blockchain_height: block.current_blockchain_height,
            };

            return Poll::Ready(Some(BroadcastMessage::NewFluffyBlock(block_mes)));
        }

        ready!(this.next_flush.as_mut().poll(cx));

        let (txs, more_available) = get_txs_to_broadcast::<N>(this.addr, this.tx_broadcast_channel);

        let next_flush = if more_available {
            // If there are more txs to broadcast then set the next flush for now so we get woken up straight away.
            Instant::now()
        } else {
            Instant::now()
                + Duration::from_secs_f64(this.diffusion_flush_dist.sample(&mut thread_rng()))
        };

        let next_flush = sleep_until(next_flush);
        this.next_flush.set(next_flush);

        if let Some(txs) = txs {
            tracing::debug!(
                "Diffusion flush timer expired, diffusing {} txs",
                txs.txs.len()
            );
            // no need to poll next_flush as we are ready now.
            Poll::Ready(Some(BroadcastMessage::NewTransaction(txs)))
        } else {
            tracing::trace!("Diffusion flush timer expired but no txs to diffuse");
            // poll next_flush now to register the waker with it
            // the waker will already be registered with the block broadcast channel.
            let _ = this.next_flush.poll(cx);
            Poll::Pending
        }
    }
}

/// Returns a list of new transactions to broadcast and a [`bool`] for if there are more txs in the queue
/// that won't fit in the current batch.
fn get_txs_to_broadcast<N: NetworkZone>(
    addr: &InternalPeerID<N::Addr>,
    broadcast_rx: &mut broadcast::Receiver<BroadcastTxInfo<N>>,
) -> (Option<NewTransactions>, bool) {
    let mut new_txs = NewTransactions {
        txs: vec![],
        dandelionpp_fluff: true,
        padding: Bytes::new(),
    };
    let mut total_size = 0;

    loop {
        match broadcast_rx.try_recv() {
            Ok(txs) => {
                if txs.received_from.is_some_and(|from| &from == addr) {
                    // If we are the one that sent this tx don't broadcast it back to us.
                    continue;
                }

                total_size += txs.tx.len();

                new_txs.txs.push(txs.tx);

                if total_size > SOFT_TX_MESSAGE_SIZE_SIZE_LIMIT {
                    return (Some(new_txs), true);
                }
            }
            Err(e) => match e {
                TryRecvError::Empty | TryRecvError::Closed => {
                    if new_txs.txs.is_empty() {
                        return (None, false);
                    }
                    return (Some(new_txs), false);
                }
                TryRecvError::Lagged(lag) => {
                    tracing::debug!(
                        "{lag} transaction broadcast messages were missed, continuing."
                    );
                    continue;
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{pin::pin, time::Duration};

    use bytes::Bytes;
    use futures::StreamExt;
    use tokio::time::timeout;
    use tower::{Service, ServiceExt};

    use cuprate_test_utils::test_netzone::TestNetZone;
    use monero_p2p::{client::InternalPeerID, BroadcastMessage, ConnectionDirection};

    use super::{init_broadcast_channels, BroadcastConfig, BroadcastRequest};

    const TEST_CONFIG: BroadcastConfig = BroadcastConfig {
        diffusion_flush_average_seconds_outbound: Duration::from_millis(100),
        diffusion_flush_average_seconds_inbound: Duration::from_millis(200),
    };

    #[tokio::test]
    async fn tx_broadcast_direction_correct() {
        let (mut brcst, outbound_mkr, inbound_mkr) =
            init_broadcast_channels::<TestNetZone<true, true, true>>(TEST_CONFIG);

        let mut outbound_stream = pin!(outbound_mkr(InternalPeerID::Unknown(1)));
        let mut inbound_stream = pin!(inbound_mkr(InternalPeerID::Unknown(1)));

        // Outbound should get 1 and 3, inbound should get 2 and 3.

        brcst
            .ready()
            .await
            .unwrap()
            .call(BroadcastRequest::Transaction {
                tx_bytes: Bytes::from_static(&[1]),
                direction: Some(ConnectionDirection::OutBound),
                received_from: None,
            })
            .await
            .unwrap();

        brcst
            .ready()
            .await
            .unwrap()
            .call(BroadcastRequest::Transaction {
                tx_bytes: Bytes::from_static(&[2]),
                direction: Some(ConnectionDirection::InBound),
                received_from: None,
            })
            .await
            .unwrap();

        brcst
            .ready()
            .await
            .unwrap()
            .call(BroadcastRequest::Transaction {
                tx_bytes: Bytes::from_static(&[3]),
                direction: None,
                received_from: None,
            })
            .await
            .unwrap();

        let match_tx = |mes, txs| match mes {
            BroadcastMessage::NewTransaction(tx) => assert_eq!(tx.txs.as_slice(), txs),
            _ => panic!("Block broadcast?"),
        };

        let next = outbound_stream.next().await.unwrap();
        let txs = [Bytes::from_static(&[1]), Bytes::from_static(&[3])];
        match_tx(next, &txs);

        let next = inbound_stream.next().await.unwrap();
        match_tx(next, &[Bytes::from_static(&[2]), Bytes::from_static(&[3])]);
    }

    #[tokio::test]
    async fn block_broadcast_sent_to_all() {
        let (mut brcst, outbound_mkr, inbound_mkr) =
            init_broadcast_channels::<TestNetZone<true, true, true>>(TEST_CONFIG);

        let mut outbound_stream = pin!(outbound_mkr(InternalPeerID::Unknown(1)));
        let mut inbound_stream = pin!(inbound_mkr(InternalPeerID::Unknown(1)));

        brcst
            .ready()
            .await
            .unwrap()
            .call(BroadcastRequest::Block {
                block_bytes: Default::default(),
                current_blockchain_height: 0,
            })
            .await
            .unwrap();

        let next = outbound_stream.next().await.unwrap();
        assert!(matches!(next, BroadcastMessage::NewFluffyBlock(_)));

        let next = inbound_stream.next().await.unwrap();
        assert!(matches!(next, BroadcastMessage::NewFluffyBlock(_)));
    }

    #[tokio::test]
    async fn tx_broadcast_skipped_for_received_from_peer() {
        let (mut brcst, outbound_mkr, inbound_mkr) =
            init_broadcast_channels::<TestNetZone<true, true, true>>(TEST_CONFIG);

        let mut outbound_stream = pin!(outbound_mkr(InternalPeerID::Unknown(1)));
        let mut outbound_stream_from = pin!(outbound_mkr(InternalPeerID::Unknown(0)));

        let mut inbound_stream = pin!(inbound_mkr(InternalPeerID::Unknown(1)));
        let mut inbound_stream_from = pin!(inbound_mkr(InternalPeerID::Unknown(0)));

        brcst
            .ready()
            .await
            .unwrap()
            .call(BroadcastRequest::Transaction {
                tx_bytes: Bytes::from_static(&[1]),
                direction: None,
                received_from: Some(InternalPeerID::Unknown(0)),
            })
            .await
            .unwrap();

        let match_tx = |mes, txs| match mes {
            BroadcastMessage::NewTransaction(tx) => assert_eq!(tx.txs.as_slice(), txs),
            _ => panic!("Block broadcast?"),
        };

        let next = outbound_stream.next().await.unwrap();
        let txs = [Bytes::from_static(&[1])];
        match_tx(next, &txs);

        let next = inbound_stream.next().await.unwrap();
        match_tx(next, &[Bytes::from_static(&[1])]);

        // Make sure the streams with the same id as the one we said sent the tx do not get the tx to broadcast.
        assert!(timeout(
            Duration::from_secs(2),
            futures::future::select(inbound_stream_from.next(), outbound_stream_from.next())
        )
        .await
        .is_err())
    }
}
