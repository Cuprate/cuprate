//! # Dandelion++ Pool
//!
//! This module contains [`DandelionPoolManager`] which is a wrapper around a backing transaction store,
//! which fully implements the dandelion++ protocol.
//!
//! The [`DandelionPoolManager`] is a middle man between a [preprocessing stage](#preprocessing-stage) and a dandelion router.
//! It handles promoting transactions in the stem state to the fluff state and setting embargo timers on stem state transactions.
//!
//! ### Preprocessing stage
//!
//! The preprocessing stage (not handled in this crate) before giving the transaction to the [`DandelionPoolManager`]
//! should handle:
//!
//! - verifying the tx.
//! - checking if we have the tx in the pool already and giving that information to the [`IncomingTxBuilder`].
//! - storing the tx in the pool, if it isn't there already.
//!
//! ### Keep Stem Transactions Hidden
//!
//! When using your handle to the backing store it must be remembered to keep transactions in the stem pool hidden.
//! So handle any requests to the tx-pool like the stem side of the pool does not exist.
use std::{
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    task::{Context, Poll},
};

use futures::{future::BoxFuture, FutureExt};
use rand_distr::Exp;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_util::{sync::PollSender, time::DelayQueue};
use tower::Service;
use tracing::Instrument;

use crate::{
    pool::manager::DandelionPoolShutDown,
    traits::{TxStoreRequest, TxStoreResponse},
    DandelionConfig, DandelionRouteReq, DandelionRouterError, State,
};

mod incoming_tx;
mod manager;

pub use incoming_tx::{IncomingTx, IncomingTxBuilder};
pub use manager::DandelionPoolManager;

/// Start the [`DandelionPoolManager`].
///
/// This function spawns the [`DandelionPoolManager`] and returns [`DandelionPoolService`] which can be used to send
/// requests to the pool.
///
/// ### Args
///
/// - `buffer_size` is the size of the channel's buffer between the [`DandelionPoolService`] and [`DandelionPoolManager`].
/// - `dandelion_router` is the router service, kept generic instead of [`DandelionRouter`](crate::DandelionRouter) to allow
///   user to customise routing functionality.
/// - `backing_pool` is the backing transaction storage service
/// - `config` is [`DandelionConfig`].
pub fn start_dandelion_pool_manager<P, R, Tx, TxId, PeerId>(
    buffer_size: usize,
    dandelion_router: R,
    backing_pool: P,
    config: DandelionConfig,
) -> DandelionPoolService<Tx, TxId, PeerId>
where
    Tx: Clone + Send + 'static,
    TxId: Hash + Eq + Clone + Send + 'static,
    PeerId: Hash + Eq + Clone + Send + 'static,
    P: Service<TxStoreRequest<TxId>, Response = TxStoreResponse<Tx>, Error = tower::BoxError>
        + Send
        + 'static,
    P::Future: Send + 'static,
    R: Service<DandelionRouteReq<Tx, PeerId>, Response = State, Error = DandelionRouterError>
        + Send
        + 'static,
    R::Future: Send + 'static,
{
    let (tx, rx) = mpsc::channel(buffer_size);

    let pool = DandelionPoolManager {
        dandelion_router,
        backing_pool,
        routing_set: JoinSet::new(),
        stem_origins: HashMap::new(),
        embargo_timers: DelayQueue::new(),
        embargo_dist: Exp::new(1.0 / config.average_embargo_timeout().as_secs_f64()).unwrap(),
        config,
        _tx: PhantomData,
    };

    let span = tracing::debug_span!("dandelion_pool");

    tokio::spawn(pool.run(rx).instrument(span));

    DandelionPoolService {
        tx: PollSender::new(tx),
    }
}

/// The dandelion pool manager service.
///
/// Used to send [`IncomingTx`]s to the [`DandelionPoolManager`]
#[derive(Clone)]
pub struct DandelionPoolService<Tx, TxId, PeerId> {
    /// The channel to [`DandelionPoolManager`].
    tx: PollSender<(IncomingTx<Tx, TxId, PeerId>, oneshot::Sender<()>)>,
}

impl<Tx, TxId, PeerId> Service<IncomingTx<Tx, TxId, PeerId>>
    for DandelionPoolService<Tx, TxId, PeerId>
where
    Tx: Clone + Send,
    TxId: Hash + Eq + Clone + Send + 'static,
    PeerId: Hash + Eq + Clone + Send + 'static,
{
    type Response = ();
    type Error = DandelionPoolShutDown;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.tx.poll_reserve(cx).map_err(|_| DandelionPoolShutDown)
    }

    fn call(&mut self, req: IncomingTx<Tx, TxId, PeerId>) -> Self::Future {
        // although the channel isn't sending anything we want to wait for the request to be handled before continuing.
        let (tx, rx) = oneshot::channel();

        let res = self
            .tx
            .send_item((req, tx))
            .map_err(|_| DandelionPoolShutDown);

        async move {
            res?;
            rx.await.expect("Oneshot dropped before response!");

            Ok(())
        }
        .boxed()
    }
}
