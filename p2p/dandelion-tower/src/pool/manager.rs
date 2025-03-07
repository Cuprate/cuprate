use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    marker::PhantomData,
    time::Duration,
};

use futures::{FutureExt, StreamExt};
use rand::prelude::*;
use rand_distr::Exp;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_util::time::DelayQueue;
use tower::{Service, ServiceExt};

use crate::{
    DandelionConfig, DandelionRouteReq, DandelionRouterError, State, TxState,
    pool::IncomingTx,
    traits::{TxStoreRequest, TxStoreResponse},
};

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error("The dandelion pool was shutdown")]
pub struct DandelionPoolShutDown;

/// The dandelion++ pool manager.
///
/// See the [module docs](super) for more.
pub struct DandelionPoolManager<P, R, Tx, TxId, PeerId> {
    /// The dandelion++ router
    pub(crate) dandelion_router: R,
    /// The backing tx storage.
    pub(crate) backing_pool: P,
    /// The set of tasks that are running the future returned from `dandelion_router`.
    pub(crate) routing_set: JoinSet<(TxId, Result<State, TxState<PeerId>>)>,

    /// The origin of stem transactions.
    pub(crate) stem_origins: HashMap<TxId, HashSet<PeerId>>,

    /// Current stem pool embargo timers.
    pub(crate) embargo_timers: DelayQueue<TxId>,
    /// The distrobution to sample to get embargo timers.
    pub(crate) embargo_dist: Exp<f64>,

    /// The d++ config.
    pub(crate) config: DandelionConfig,

    pub(crate) _tx: PhantomData<Tx>,
}

impl<P, R, Tx, TxId, PeerId> DandelionPoolManager<P, R, Tx, TxId, PeerId>
where
    Tx: Clone + Send,
    TxId: Hash + Eq + Clone + Send + 'static,
    PeerId: Hash + Eq + Clone + Send + 'static,
    P: Service<TxStoreRequest<TxId>, Response = TxStoreResponse<Tx>, Error = tower::BoxError>,
    P::Future: Send + 'static,
    R: Service<DandelionRouteReq<Tx, PeerId>, Response = State, Error = DandelionRouterError>,
    R::Future: Send + 'static,
{
    /// Adds a new embargo timer to the running timers, with a duration pulled from [`Self::embargo_dist`]
    fn add_embargo_timer_for_tx(&mut self, tx_id: TxId) {
        let embargo_timer = self.embargo_dist.sample(&mut thread_rng());
        tracing::debug!(
            "Setting embargo timer for stem tx: {} seconds.",
            embargo_timer
        );

        self.embargo_timers
            .insert(tx_id, Duration::from_secs_f64(embargo_timer));
    }

    /// Stems the tx, setting the stem origin, if it wasn't already set.
    ///
    /// This function does not add the tx to the backing pool.
    async fn stem_tx(
        &mut self,
        tx: Tx,
        tx_id: TxId,
        from: Option<PeerId>,
    ) -> Result<(), tower::BoxError> {
        if let Some(peer) = &from {
            self.stem_origins
                .entry(tx_id.clone())
                .or_default()
                .insert(peer.clone());
        }

        let state = from.map_or(TxState::Local, |from| TxState::Stem { from });

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

    /// Fluffs a tx, does not add the tx to the tx pool.
    async fn fluff_tx(&mut self, tx: Tx, tx_id: TxId) -> Result<(), tower::BoxError> {
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

    /// Function to handle an [`IncomingTx`].
    async fn handle_incoming_tx(
        &mut self,
        tx: Tx,
        tx_state: TxState<PeerId>,
        tx_id: TxId,
    ) -> Result<(), tower::BoxError> {
        match tx_state {
            TxState::Stem { from } => {
                if self
                    .stem_origins
                    .get(&tx_id)
                    .is_some_and(|peers| peers.contains(&from))
                {
                    tracing::debug!("Received stem tx twice from same peer, fluffing it");
                    // The same peer sent us a tx twice, fluff it.
                    self.promote_and_fluff_tx(tx_id).await?;
                } else {
                    // This could be a new tx or it could have already been stemed, but we still stem it again
                    // unless the same peer sends us a tx twice.
                    tracing::debug!("Steming incoming tx");
                    self.stem_tx(tx, tx_id.clone(), Some(from)).await?;
                    self.add_embargo_timer_for_tx(tx_id);
                }
            }
            TxState::Fluff => {
                tracing::debug!("Fluffing incoming tx");
                self.fluff_tx(tx, tx_id).await?;
            }
            TxState::Local => {
                tracing::debug!("Steming local transaction");
                self.stem_tx(tx, tx_id.clone(), None).await?;
                self.add_embargo_timer_for_tx(tx_id);
            }
        }

        Ok(())
    }

    /// Promotes a tx to the clear pool.
    async fn promote_tx(&mut self, tx_id: TxId) -> Result<(), tower::BoxError> {
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
    async fn promote_and_fluff_tx(&mut self, tx_id: TxId) -> Result<(), tower::BoxError> {
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
    async fn get_tx_from_pool(&mut self, tx_id: TxId) -> Result<Option<Tx>, tower::BoxError> {
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

    /// Starts the [`DandelionPoolManager`].
    pub(crate) async fn run(
        mut self,
        mut rx: mpsc::Receiver<(IncomingTx<Tx, TxId, PeerId>, oneshot::Sender<()>)>,
    ) {
        tracing::debug!("Starting dandelion++ tx-pool, config: {:?}", self.config);

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

                    let Some((IncomingTx { tx, tx_id, routing_state }, res_tx)) = req else {
                        return;
                    };

                    if let Err(e) = self.handle_incoming_tx(tx, routing_state, tx_id).await {
                        #[expect(clippy::let_underscore_must_use, reason = "dropped receivers can be ignored")]
                        let _ = res_tx.send(());

                        tracing::error!("Error handling transaction in dandelion pool: {e}");
                        return;
                    }

                    #[expect(clippy::let_underscore_must_use)]
                    let _ = res_tx.send(());
                }
            }
        }
    }
}
