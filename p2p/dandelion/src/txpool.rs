//! # Dandelion++ Pool
//!
//! This module contains [`DandelionPool`] which is a thin wrapper around a backing transaction store,
//! which fully implements the dandelion++ protocol.
//!
//! ### How To Get Txs From [`DandelionPool`].
//!
//! [`DandelionPool`] does not provide a full tx-pool API. You cannot retrieve transactions from it or
//! check what transactions are in it, to do this you must keep a handle to the backing transaction store
//! yourself.
//!
//! The reason for this is, the [`DandelionPool`] will only itself be passing these requests onto the backing
//! pool, so it makes sense to remove the "middle man".
//!
//! ### Keep Stem Transactions Hidden
//!
//! When using your handle to the backing store it must be remembered to keep transactions in the stem pool hidden.
//! So handle any requests to the tx-pool like the stem side of the pool does not exist.
//!
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    hash::Hash,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures::{FutureExt, StreamExt};
use rand::prelude::*;
use rand_distr::Exp;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_util::{sync::PollSender, time::DelayQueue};
use tower::{Service, ServiceExt};
use tracing::Instrument;

use crate::{
    traits::{TxStoreRequest, TxStoreResponse},
    DandelionConfig, DandelionRouteReq, DandelionRouterError, State, TxState,
};

/// Start the [`DandelionPool`].
///
/// This function spawns the [`DandelionPool`] and returns [`DandelionPoolService`] which can be used to send
/// requests to the pool.
///
/// ### Args
///
/// - `buffer_size` is the size of the channel's buffer between the [`DandelionPoolService`] and [`DandelionPool`].
/// - `dandelion_router` is the router service, kept generic instead of [`DandelionRouter`](crate::DandelionRouter) to allow
/// user to customise routing functionality.
/// - `backing_pool` is the backing transaction storage service
/// - `config` is [`DandelionConfig`].
pub fn start_dandelion_pool<P, R, Tx, TxID, PID>(
    buffer_size: usize,
    dandelion_router: R,
    backing_pool: P,
    config: DandelionConfig,
) -> DandelionPoolService<Tx, TxID, PID>
where
    Tx: Clone + Send + 'static,
    TxID: Hash + Eq + Clone + Send + 'static,
    PID: Hash + Eq + Clone + Send + 'static,
    P: Service<
            TxStoreRequest<Tx, TxID>,
            Response = TxStoreResponse<Tx, TxID>,
            Error = tower::BoxError,
        > + Send
        + 'static,
    P::Future: Send + 'static,
    R: Service<DandelionRouteReq<Tx, PID>, Response = State, Error = DandelionRouterError>
        + Send
        + 'static,
    R::Future: Send + 'static,
{
    let (tx, rx) = mpsc::channel(buffer_size);

    let pool = DandelionPool {
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

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error("The dandelion pool was shutdown")]
pub struct DandelionPoolShutDown;

/// An incoming transaction for the [`DandelionPool`] to handle.
///
/// Users may notice there is no way to check if the dandelion-pool wants a tx according to an inventory message like seen
/// in Bitcoin, only having a request for a full tx. Users should look in the *public* backing pool to handle inv messages,
/// and request txs even if they are in the stem pool.
pub struct IncomingTx<Tx, TxID, PID> {
    /// The transaction.
    ///
    /// It is recommended to put this in an [`Arc`](std::sync::Arc) as it needs to be cloned to send to the backing
    /// tx pool and [`DandelionRouter`](crate::DandelionRouter)
    pub tx: Tx,
    /// The transaction ID.
    pub tx_id: TxID,
    /// The routing state of this transaction.
    pub tx_state: TxState<PID>,
}

/// The dandelion tx pool service.
#[derive(Clone)]
pub struct DandelionPoolService<Tx, TxID, PID> {
    /// The channel to [`DandelionPool`].
    tx: PollSender<(IncomingTx<Tx, TxID, PID>, oneshot::Sender<()>)>,
}

impl<Tx, TxID, PID> Service<IncomingTx<Tx, TxID, PID>> for DandelionPoolService<Tx, TxID, PID>
where
    Tx: Clone + Send,
    TxID: Hash + Eq + Clone + Send + 'static,
    PID: Hash + Eq + Clone + Send + 'static,
{
    type Response = ();
    type Error = DandelionPoolShutDown;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.tx.poll_reserve(cx).map_err(|_| DandelionPoolShutDown)
    }

    fn call(&mut self, req: IncomingTx<Tx, TxID, PID>) -> Self::Future {
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

/// The dandelion++ tx pool.
///
/// See the [module docs](self) for more.
pub struct DandelionPool<P, R, Tx, TxID, PID> {
    /// The dandelion++ router
    dandelion_router: R,
    /// The backing tx storage.
    backing_pool: P,
    /// The set of tasks that are running the future returned from `dandelion_router`.
    routing_set: JoinSet<(TxID, Result<State, TxState<PID>>)>,

    /// The origin of stem transactions.
    stem_origins: HashMap<TxID, HashSet<PID>>,

    /// Current stem pool embargo timers.
    embargo_timers: DelayQueue<TxID>,
    /// The distrobution to sample to get embargo timers.
    embargo_dist: Exp<f64>,

    /// The d++ config.
    config: DandelionConfig,

    _tx: PhantomData<Tx>,
}

impl<P, R, Tx, TxID, PID> DandelionPool<P, R, Tx, TxID, PID>
where
    Tx: Clone + Send,
    TxID: Hash + Eq + Clone + Send + 'static,
    PID: Hash + Eq + Clone + Send + 'static,
    P: Service<
        TxStoreRequest<Tx, TxID>,
        Response = TxStoreResponse<Tx, TxID>,
        Error = tower::BoxError,
    >,
    P::Future: Send + 'static,
    R: Service<DandelionRouteReq<Tx, PID>, Response = State, Error = DandelionRouterError>,
    R::Future: Send + 'static,
{
    /// Stores the tx in the backing pools stem pool, setting the embargo timer, stem origin and steming the tx.
    async fn store_tx_and_stem(
        &mut self,
        tx: Tx,
        tx_id: TxID,
        from: Option<PID>,
    ) -> Result<(), tower::BoxError> {
        self.backing_pool
            .ready()
            .await?
            .call(TxStoreRequest::Store(
                tx.clone(),
                tx_id.clone(),
                State::Stem,
            ))
            .await?;

        let embargo_timer = self.embargo_dist.sample(&mut thread_rng());
        tracing::debug!(
            "Setting embargo timer for stem tx: {} seconds.",
            embargo_timer
        );
        self.embargo_timers
            .insert(tx_id.clone(), Duration::from_secs_f64(embargo_timer));

        self.stem_tx(tx, tx_id, from).await
    }

    /// Stems the tx, setting the stem origin, if it wasn't already set.
    ///
    /// This function does not add the tx to the backing pool.
    async fn stem_tx(
        &mut self,
        tx: Tx,
        tx_id: TxID,
        from: Option<PID>,
    ) -> Result<(), tower::BoxError> {
        if let Some(peer) = &from {
            self.stem_origins
                .entry(tx_id.clone())
                .or_default()
                .insert(peer.clone());
        }

        let state = from
            .map(|from| TxState::Stem { from })
            .unwrap_or(TxState::Local);

        let fut = self
            .dandelion_router
            .ready()
            .await?
            .call(DandelionRouteReq {
                tx,
                state: state.clone(),
            });

        self.routing_set
            .spawn(fut.map(|res| (tx_id, res.map_err(|_| state))));
        Ok(())
    }

    /// Stores the tx in the backing pool and fluffs the tx, removing the stem data for this tx.
    async fn store_and_fluff_tx(&mut self, tx: Tx, tx_id: TxID) -> Result<(), tower::BoxError> {
        // fluffs the tx first to prevent timing attacks where we could fluff at different average times
        // depending on if the tx was in the stem pool already or not.
        // Massively overkill but this is a minimal change.
        self.fluff_tx(tx.clone(), tx_id.clone()).await?;

        // Remove the tx from the maps used during the stem phase.
        self.stem_origins.remove(&tx_id);

        self.backing_pool
            .ready()
            .await?
            .call(TxStoreRequest::Store(tx, tx_id, State::Fluff))
            .await?;

        // The key for this is *Not* the tx_id, it is given on insert, so just keep the timer in the
        // map. These timers should be relatively short, so it shouldn't be a problem.
        //self.embargo_timers.try_remove(&tx_id);

        Ok(())
    }

    /// Fluffs a tx, does not add the tx to the tx pool.
    async fn fluff_tx(&mut self, tx: Tx, tx_id: TxID) -> Result<(), tower::BoxError> {
        let fut = self
            .dandelion_router
            .ready()
            .await?
            .call(DandelionRouteReq {
                tx,
                state: TxState::Fluff,
            });

        self.routing_set
            .spawn(fut.map(|res| (tx_id, res.map_err(|_| TxState::Fluff))));
        Ok(())
    }

    /// Function to handle an incoming [`DandelionPoolRequest::IncomingTx`].
    async fn handle_incoming_tx(
        &mut self,
        tx: Tx,
        tx_state: TxState<PID>,
        tx_id: TxID,
    ) -> Result<(), tower::BoxError> {
        let TxStoreResponse::Contains(have_tx) = self
            .backing_pool
            .ready()
            .await?
            .call(TxStoreRequest::Contains(tx_id.clone()))
            .await?
        else {
            panic!("Backing tx pool responded with wrong response for request.");
        };
        // If we have already fluffed this tx then we don't need to do anything.
        if have_tx == Some(State::Fluff) {
            tracing::debug!("Already fluffed incoming tx, ignoring.");
            return Ok(());
        }

        match tx_state {
            TxState::Stem { from } => {
                if self
                    .stem_origins
                    .get(&tx_id)
                    .is_some_and(|peers| peers.contains(&from))
                {
                    tracing::debug!("Received stem tx twice from same peer, fluffing it");
                    // The same peer sent us a tx twice, fluff it.
                    self.promote_and_fluff_tx(tx_id).await
                } else {
                    // This could be a new tx or it could have already been stemed, but we still stem it again
                    // unless the same peer sends us a tx twice.
                    tracing::debug!("Steming incoming tx");
                    self.store_tx_and_stem(tx, tx_id, Some(from)).await
                }
            }
            TxState::Fluff => {
                tracing::debug!("Fluffing incoming tx");
                self.store_and_fluff_tx(tx, tx_id).await
            }
            TxState::Local => {
                // If we have already stemed this tx then nothing to do.
                if have_tx.is_some() {
                    tracing::debug!("Received a local tx that we already have, skipping");
                    return Ok(());
                }
                tracing::debug!("Steming local transaction");
                self.store_tx_and_stem(tx, tx_id, None).await
            }
        }
    }

    /// Promotes a tx to the clear pool.
    async fn promote_tx(&mut self, tx_id: TxID) -> Result<(), tower::BoxError> {
        // Remove the tx from the maps used during the stem phase.
        self.stem_origins.remove(&tx_id);

        // The key for this is *Not* the tx_id, it is given on insert, so just keep the timer in the
        // map. These timers should be relatively short, so it shouldn't be a problem.
        //self.embargo_timers.try_remove(&tx_id);

        self.backing_pool
            .ready()
            .await?
            .call(TxStoreRequest::Promote(tx_id))
            .await?;

        Ok(())
    }

    /// Promotes a tx to the public fluff pool and fluffs the tx.
    async fn promote_and_fluff_tx(&mut self, tx_id: TxID) -> Result<(), tower::BoxError> {
        tracing::debug!("Promoting transaction to public pool and fluffing it.");

        let TxStoreResponse::Transaction(tx) = self
            .backing_pool
            .ready()
            .await?
            .call(TxStoreRequest::Get(tx_id.clone()))
            .await?
        else {
            panic!("Backing tx pool responded with wrong response for request.");
        };

        let Some((tx, state)) = tx else {
            tracing::debug!("Could not find tx, skipping.");
            return Ok(());
        };

        if state == State::Fluff {
            tracing::debug!("Transaction already fluffed, skipping.");
            return Ok(());
        }

        self.promote_tx(tx_id.clone()).await?;
        self.fluff_tx(tx, tx_id).await
    }

    /// Returns a tx stored in the fluff _OR_ stem pool.
    async fn get_tx_from_pool(&mut self, tx_id: TxID) -> Result<Option<Tx>, tower::BoxError> {
        let TxStoreResponse::Transaction(tx) = self
            .backing_pool
            .ready()
            .await?
            .call(TxStoreRequest::Get(tx_id))
            .await?
        else {
            panic!("Backing tx pool responded with wrong response for request.");
        };

        Ok(tx.map(|tx| tx.0))
    }

    /// Starts the [`DandelionPool`].
    async fn run(
        mut self,
        mut rx: mpsc::Receiver<(IncomingTx<Tx, TxID, PID>, oneshot::Sender<()>)>,
    ) {
        tracing::debug!("Starting dandelion++ tx-pool, config: {:?}", self.config);

        // On start up we just fluff all txs left in the stem pool.
        let Ok(TxStoreResponse::IDs(ids)) = (&mut self.backing_pool)
            .oneshot(TxStoreRequest::IDsInStemPool)
            .await
        else {
            tracing::error!("Failed to get transactions in stem pool.");
            return;
        };

        tracing::debug!(
            "Fluffing {} txs that are currently in the stem pool",
            ids.len()
        );

        for id in ids {
            if let Err(e) = self.promote_and_fluff_tx(id).await {
                tracing::error!("Failed to fluff tx in the stem pool at start up, {e}.");
                return;
            }
        }

        loop {
            tracing::trace!("Waiting for next event.");
            tokio::select! {
                // biased to handle current txs before routing new ones.
                biased;
                Some(fired) = self.embargo_timers.next() => {
                    tracing::debug!("Embargo timer fired, did not see stem tx in time.");

                    let tx_id = fired.into_inner();
                    if let Err(e) = self.promote_and_fluff_tx(tx_id).await {
                        tracing::error!("Error handling fired embargo timer: {e}");
                        return;
                    }
                }
                Some(Ok((tx_id, res))) = self.routing_set.join_next() => {
                    tracing::trace!("Received d++ routing result.");

                    let res = match res {
                        Ok(State::Fluff) => {
                            tracing::debug!("Transaction was fluffed upgrading it to the public pool.");
                            self.promote_tx(tx_id).await
                        }
                        Err(tx_state) => {
                            tracing::debug!("Error routing transaction, trying again.");

                            match self.get_tx_from_pool(tx_id.clone()).await {
                                Ok(Some(tx)) => match tx_state {
                                    TxState::Fluff => self.fluff_tx(tx, tx_id).await,
                                    TxState::Stem { from } => self.stem_tx(tx, tx_id, Some(from)).await,
                                    TxState::Local => self.stem_tx(tx, tx_id, None).await,
                                }
                                Err(e) => Err(e),
                                _ => continue,
                            }
                        }
                        Ok(State::Stem) => continue,
                    };

                    if let Err(e) = res {
                        tracing::error!("Error handling transaction routing return: {e}");
                        return;
                    }
                }
                req = rx.recv() => {
                    tracing::debug!("Received new tx to route.");

                    let Some((IncomingTx { tx, tx_state, tx_id }, res_tx)) = req else {
                        return;
                    };

                    if let Err(e) = self.handle_incoming_tx(tx, tx_state, tx_id).await {
                        let _ = res_tx.send(());

                        tracing::error!("Error handling transaction in dandelion pool: {e}");
                        return;
                    }
                    let _ = res_tx.send(());

                }
            }
        }
    }
}
