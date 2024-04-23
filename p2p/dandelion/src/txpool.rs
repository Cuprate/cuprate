use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    marker::PhantomData,
    time::Duration,
};

use futures::{FutureExt, StreamExt};
use rand::prelude::*;
use rand_distr::Exp;
use tokio::{sync::mpsc, task::JoinSet};
use tokio_util::time::DelayQueue;
use tower::{Service, ServiceExt};

use crate::{
    traits::{TxStoreRequest, TxStoreResponse},
    DandelionConfig, DandelionRouteReq, DandelionRouterError, State, TxState,
};

/// A request to the dandelion tx pool.
///
/// This enum does not contain requests to get tx data from the pool(s), this is to reduce the amount
/// of requests that get sent to the [`DandelionPool`]. Users should hold a handle to the backing tx storage
/// to get transaction data from the pool.
///
/// It MUST be remembered to not expose the private stem pool to peers.
///
/// Users may notice there is no way to check if the dandelion-pool wants a tx according to an inventory message like seen
/// in Bitcoin, only having a request for a full tx. Users should look in the *public* backing pool to handle inv messages,
/// and request txs even if they are in the stem pool.
pub enum DandelionPoolRequest<Tx, TxID, PID> {
    /// An incoming tx.
    IncomingTx {
        /// The transaction.
        tx: Tx,
        /// The transaction ID.
        tx_id: TxID,
        /// The routing state of this transaction.
        tx_state: TxState<PID>,
    },
}

pub struct DandelionPool<P, R, Tx, TxID, PID> {
    /// The dandelion++ router
    dandelion_router: R,
    /// The backing tx storage.
    backing_pool: P,

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
    Tx: Clone,
    TxID: Hash + Eq + Clone + Send + 'static,
    PID: Hash + Eq + Clone + Send + 'static,
    P: Service<TxStoreRequest<Tx, TxID>, Response = TxStoreResponse<Tx>, Error = tower::BoxError>,
    P::Future: Send + 'static,
    R: Service<DandelionRouteReq<Tx, PID>, Response = State, Error = DandelionRouterError>,
    R::Future: Send + 'static,
{
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
        self.embargo_timers
            .insert(tx_id.clone(), Duration::from_secs_f64(embargo_timer));

        self.stem_tx(tx, tx_id, from).await
    }

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

    async fn fluff_tx(&mut self, tx: Tx, tx_id: TxID) -> Result<(), tower::BoxError> {
        self.backing_pool
            .ready()
            .await?
            .call(TxStoreRequest::Store(
                tx.clone(),
                tx_id.clone(),
                State::Fluff,
            ))
            .await?;

        // Remove the tx from the maps used during the stem phase.
        self.stem_origins.remove(&tx_id);

        // The key for this is *Not* the tx_id, it is given on insert, so just keep the timer in the
        // map. These timers should be relatively short, so it shouldn't be a problem.
        //self.embargo_timers.try_remove(&tx_id);

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
            return Ok(());
        }

        match tx_state {
            TxState::Stem { from } => {
                if self
                    .stem_origins
                    .get(&tx_id)
                    .is_some_and(|peers| peers.contains(&from))
                {
                    self.fluff_tx(tx, tx_id).await
                } else {
                    self.store_tx_and_stem(tx, tx_id, Some(from)).await
                }
            }
            TxState::Fluff => self.fluff_tx(tx, tx_id).await,
            TxState::Local => {
                // If we have already stemed this tx then nothing to do.
                if have_tx.is_some() {
                    return Ok(());
                }

                self.store_tx_and_stem(tx, tx_id, None).await
            }
        }
    }

    async fn promote_tx(&mut self, tx_id: &TxID) -> Result<(), tower::BoxError> {
        self.backing_pool
            .ready()
            .await?
            .call(TxStoreRequest::Promote(tx_id.clone()))
            .await?;

        // Remove the tx from the maps used during the stem phase.
        self.stem_origins.remove(tx_id);

        // The key for this is *Not* the tx_id, it is given on insert, so just keep the timer in the
        // map. These timers should be relatively short, so it shouldn't be a problem.
        //self.embargo_timers.try_remove(&tx_id);

        Ok(())
    }

    async fn promote_and_fluff_tx(&mut self, tx_id: TxID) -> Result<(), tower::BoxError> {
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
            return Ok(());
        };

        if state == State::Fluff {
            return Ok(());
        }

        self.fluff_tx(tx, tx_id).await
    }

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

    async fn run(mut self, mut rx: mpsc::Receiver<DandelionPoolRequest<Tx, TxID, PID>>) {
        // TODO: handle txs currently in stem pool.

        loop {
            tokio::select! {
                biased;
                Some(fired) = self.embargo_timers.next() => {
                    let tx_id = fired.into_inner();
                    if let Err(e) = self.promote_and_fluff_tx(tx_id).await {
                        tracing::error!("Error handling fired embargo timer: {e}");
                        return;
                    }
                }
                Some(Ok((tx_id, res))) = self.routing_set.join_next() => {
                    let res = match res {
                        Ok(State::Fluff) => {
                            self.promote_tx(&tx_id).await
                        }
                        Err(tx_state) => {
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
                    let Some(DandelionPoolRequest::IncomingTx { tx, tx_state, tx_id }) = req else {
                        return;
                    };

                    if let Err(e) = self.handle_incoming_tx(tx, tx_state, tx_id).await {
                        tracing::error!("Error handling transaction in dandelion pool: {e}");
                        return;
                    }
                }
            }
        }
    }
}
