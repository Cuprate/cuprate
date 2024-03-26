//! # Broadcast Router
//!
//! This module handles broadcasting messages to multiple peers with the [`BroadcastSvc`].
use std::{
    collections::HashSet,
    future::{ready, Future, Ready},
    pin::{pin, Pin},
    sync::Arc,
    task::{ready, Context, Poll},
    time::Duration,
};

use bytes::Bytes;
use futures::Stream;
use rand::prelude::*;
use rand_distr::Poisson;
use tokio::{
    sync::{
        broadcast::{self, error::TryRecvError},
        watch,
    },
    time::{sleep_until, Instant, Sleep},
};
use tokio_stream::wrappers::WatchStream;
use tokio_util::sync::CancellationToken;
use tower::Service;

use monero_p2p::{client::InternalPeerID, BroadcastMessage, ConnectionDirection, NetworkZone};
use monero_wire::{
    common::{BlockCompleteEntry, TransactionBlobs},
    protocol::{NewFluffyBlock, NewTransactions},
};

use crate::constants::{
    DIFFUSION_FLUSH_AVERAGE_SECONDS_INBOUND, DIFFUSION_FLUSH_AVERAGE_SECONDS_OUTBOUND,
    DIFFUSION_POISSON_SECOND_FRACTION, MAX_TXS_IN_BROADCAST_CHANNEL,
    SOFT_TX_MESSAGE_SIZE_SIZE_LIMIT,
};

#[derive(Debug, Clone)]
pub struct BroadcastConfig {
    pub diffusion_flush_average_seconds_outbound: f32,
    pub diffusion_flush_average_seconds_inbound: f32,
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
    config: &BroadcastConfig,
) -> (
    BroadcastSvc<N>,
    impl Fn(InternalPeerID<N::Addr>) -> BroadcastMessageStream<N> + Clone + Send + 'static,
    impl Fn(InternalPeerID<N::Addr>) -> BroadcastMessageStream<N> + Clone + Send + 'static,
) {
    // See [`DIFFUSION_POISSON_SECOND_FRACTION`] for details on this.
    let outbound_dist = Poisson::new(
        config.diffusion_flush_average_seconds_outbound * DIFFUSION_POISSON_SECOND_FRACTION,
    )
    .unwrap();
    let inbound_dist = Poisson::new(
        config.diffusion_flush_average_seconds_inbound * DIFFUSION_POISSON_SECOND_FRACTION,
    )
    .unwrap();

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
        CloneableBroadcastRecover(tx_broadcast_channel_outbound_receiver);
    let tx_channel_inbound_receiver_wrapped =
        CloneableBroadcastRecover(tx_broadcast_channel_inbound_receiver);

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
        tx_bytes: Bytes,
        direction: Option<ConnectionDirection>,
        skip_peers: Arc<std::sync::Mutex<HashSet<InternalPeerID<N::Addr>>>>,
    },
}

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
                    "queuing block at chain height {} for broadcast",
                    current_blockchain_height
                );

                self.new_block_watch.send_replace(NewBlockInfo {
                    block_bytes,
                    current_blockchain_height,
                });
            }
            BroadcastRequest::Transaction {
                tx_bytes,
                skip_peers,
                direction,
            } => {
                let nex_tx_info = BroadcastTxInfo {
                    tx: tx_bytes,
                    skip_peers,
                    // TODO: pass this in.
                    cancel: CancellationToken::new(),
                };

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
struct CloneableBroadcastRecover<T: Clone>(broadcast::Receiver<T>);

impl<T: Clone> Clone for CloneableBroadcastRecover<T> {
    fn clone(&self) -> Self {
        Self(self.0.resubscribe())
    }
}

#[derive(Clone)]
struct NewBlockInfo {
    /// The block.
    block_bytes: Bytes,
    /// The current chain height - will be 1 more than the blocks' height.
    current_blockchain_height: u64,
}

#[derive(Clone)]
struct BroadcastTxInfo<N: NetworkZone> {
    tx: Bytes,
    skip_peers: Arc<std::sync::Mutex<HashSet<InternalPeerID<N::Addr>>>>,
    cancel: CancellationToken,
}

#[pin_project::pin_project]
pub struct BroadcastMessageStream<N: NetworkZone> {
    addr: InternalPeerID<N::Addr>,

    #[pin]
    new_block_watch: WatchStream<NewBlockInfo>,
    tx_broadcast_channel: broadcast::Receiver<BroadcastTxInfo<N>>,

    diffusion_flush_dist: Poisson<f32>,
    #[pin]
    next_flush: Sleep,
}

impl<N: NetworkZone> BroadcastMessageStream<N> {
    fn new(
        addr: InternalPeerID<N::Addr>,
        diffusion_flush_dist: Poisson<f32>,
        new_block_watch: watch::Receiver<NewBlockInfo>,
        tx_broadcast_channel: broadcast::Receiver<BroadcastTxInfo<N>>,
    ) -> Self {
        let next_flush = next_diffusion_flush(&diffusion_flush_dist);

        Self {
            addr,
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

        if let Poll::Ready(res) = this.new_block_watch.poll_next(cx) {
            let Some(block) = res else {
                return Poll::Ready(None);
            };

            let block_mes = NewFluffyBlock {
                b: BlockCompleteEntry {
                    pruned: false,
                    block: block.block_bytes,
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
            next_diffusion_flush(this.diffusion_flush_dist)
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

/// Returns the [`Instant`] that the next tx diffusion flush should occur at.
fn next_diffusion_flush(diffusion_flush_dist: &Poisson<f32>) -> Instant {
    let now = Instant::now();

    let val = diffusion_flush_dist.sample(&mut thread_rng());

    // See [`DIFFUSION_POISSON_SECOND_FRACTION`] for details on this.
    let seconds = Duration::from_secs_f32(val / DIFFUSION_POISSON_SECOND_FRACTION);

    tracing::trace!("next diffusion flush in {} seconds", seconds.as_secs_f32());

    now + seconds
}

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
                if txs.cancel.is_cancelled() || txs.skip_peers.lock().unwrap().contains(addr) {
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
                        "{} transaction broadcast messages were missed, continuing.",
                        lag
                    );
                    continue;
                }
            },
        }
    }
}
