use std::future::Future;
use std::ops::Div;
use std::{
    collections::{HashMap, HashSet},
    convert,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::future::BoxFuture;
use futures::TryFutureExt;
use futures::{
    channel::{mpsc, oneshot},
    stream::FuturesUnordered,
    Stream,
};
use futures::{FutureExt, SinkExt};
use tokio::{sync::oneshot::error::TryRecvError, task::JoinHandle};
use tower::{
    discover::{Change, Discover},
    load::Load,
    BoxError, Service,
};

use monero_wire::{NetworkAddress, PeerID};

use super::{unready_service::UnreadyError, UnreadyService};
use crate::{
    peer::LoadTrackedClient,
    protocol::{InternalMessageRequest, InternalMessageResponse},
    Config,
};

/// A signal sent by the [`PeerSet`] when it has no ready peers, and gets a request from Zebra.
///
/// In response to this signal, the crawler tries to open more peer connections.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MorePeers;

/// A signal sent by the [`PeerSet`] to cancel a [`Client`][1]'s current request
/// or response.
///
/// When it receives this signal, the [`Client`][1] stops processing and exits.
///
/// [1]: crate::peer::Client
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CancelClientWork;

/// A [`tower::Service`] that abstractly represents "the rest of the network".
///
/// # Security
///
/// The `Discover::Key` must be the transient remote address of each peer. This
/// address may only be valid for the duration of a single connection. (For
/// example, inbound connections have an ephemeral remote port, and proxy
/// connections have an ephemeral local or proxy port.)
///
/// Otherwise, malicious peers could interfere with other peers' `PeerSet` state.
pub struct PeerSet<D>
where
    D: Discover<Key = PeerID, Service = LoadTrackedClient> + Unpin,
    D::Error: Into<BoxError>,
{
    /// Peer Tracking: New Peers
    ///
    /// Provides new and deleted peer [`Change`]s to the peer set,
    /// via the [`Discover`] trait implementation.
    discover: D,

    /// A channel that asks the peer crawler task to connect to more peers.
    demand_signal: mpsc::Sender<MorePeers>,

    /// Peer Tracking: Ready Peers
    ///
    /// Connected peers that are ready to receive requests from Zebra,
    /// or send requests to Zebra.
    ready_services: HashMap<D::Key, D::Service>,

    /// Peer Tracking: Busy Peers
    ///
    /// Connected peers that are handling a Zebra request,
    /// or Zebra is handling one of their requests.
    unready_services: FuturesUnordered<UnreadyService<D::Key, D::Service, InternalMessageRequest>>,

    /// Channels used to cancel the request that an unready service is doing.
    cancel_handles: HashMap<D::Key, oneshot::Sender<CancelClientWork>>,

    /// The configured limit for inbound and outbound connections.
    ///
    /// The peer set panics if this size is exceeded.
    /// If that happens, our connection limit code has a bug.
    peerset_total_connection_limit: usize,

    // Background Tasks
    //
    /// Channel for passing ownership of tokio JoinHandles from PeerSet's background tasks
    ///
    /// The join handles passed into the PeerSet are used populate the `guards` member
    handle_rx: tokio::sync::oneshot::Receiver<Vec<JoinHandle<Result<(), BoxError>>>>,

    /// Unordered set of handles to background tasks associated with the `PeerSet`
    ///
    /// These guards are checked for errors as part of `poll_ready` which lets
    /// the `PeerSet` propagate errors from background tasks back to the user
    guards: FuturesUnordered<JoinHandle<Result<(), BoxError>>>,
}

impl<D> PeerSet<D>
where
    D: Discover<Key = PeerID, Service = LoadTrackedClient> + Unpin,
    D::Error: Into<BoxError>,
{
    /// Construct a peerset which uses `discover` to manage peer connections.
    ///
    /// Arguments:
    /// - `config`: configures the peer set connection limit;
    /// - `discover`: handles peer connects and disconnects;
    /// - `demand_signal`: requests more peers when all peers are busy (unready);
    /// - `handle_rx`: receives background task handles,
    ///                monitors them to make sure they're still running,
    ///                and shuts down all the tasks as soon as one task exits;
    pub fn new(
        config: &Config,
        discover: D,
        demand_signal: mpsc::Sender<MorePeers>,
        handle_rx: tokio::sync::oneshot::Receiver<Vec<JoinHandle<Result<(), BoxError>>>>,
    ) -> Self {
        Self {
            // New peers
            discover,
            demand_signal,

            // Ready peers
            ready_services: HashMap::new(),

            // Busy peers
            unready_services: FuturesUnordered::new(),
            cancel_handles: HashMap::new(),

            peerset_total_connection_limit: config.peerset_total_connection_limit(),

            // Background tasks
            handle_rx,
            guards: futures::stream::FuturesUnordered::new(),
        }
    }

    /// Receive background tasks, if they've been sent on the channel,
    /// but not consumed yet.
    ///
    /// Returns a result representing the current task state,
    /// or `None` if the background tasks should be polled to check their state.
    fn receive_tasks_if_needed(&mut self) -> Option<Result<(), BoxError>> {
        if self.guards.is_empty() {
            match self.handle_rx.try_recv() {
                // The tasks haven't been sent yet.
                Err(TryRecvError::Empty) => Some(Ok(())),

                // The tasks have been sent, but not consumed.
                Ok(handles) => {
                    // Currently, the peer set treats an empty background task set as an error.
                    //
                    // TODO: refactor `handle_rx` and `guards` into an enum
                    //       for the background task state: Waiting/Running/Shutdown.
                    assert!(
                        !handles.is_empty(),
                        "the peer set requires at least one background task"
                    );

                    self.guards.extend(handles);

                    None
                }

                // The tasks have been sent and consumed, but then they exited.
                //
                // Correctness: the peer set must receive at least one task.
                //
                // TODO: refactor `handle_rx` and `guards` into an enum
                //       for the background task state: Waiting/Running/Shutdown.
                Err(TryRecvError::Closed) => {
                    Some(Err("all peer set background tasks have exited".into()))
                }
            }
        } else {
            None
        }
    }

    /// Check background task handles to make sure they're still running.
    ///
    /// If any background task exits, shuts down all other background tasks,
    /// and returns an error.
    fn poll_background_errors(&mut self, cx: &mut Context) -> Result<(), BoxError> {
        if let Some(result) = self.receive_tasks_if_needed() {
            return result;
        }

        match Pin::new(&mut self.guards).poll_next(cx) {
            // All background tasks are still running.
            Poll::Pending => Ok(()),

            Poll::Ready(Some(res)) => {
                tracing::info!(
                    background_tasks = %self.guards.len(),
                    "a peer set background task exited, shutting down other peer set tasks"
                );

                self.shut_down_tasks_and_channels();

                // Flatten the join result and inner result,
                // then turn Ok() task exits into errors.
                res.map_err(Into::into)
                    // TODO: replace with Result::flatten when it stabilises (#70142)
                    .and_then(convert::identity)
                    .and(Err("a peer set background task exited".into()))
            }

            Poll::Ready(None) => {
                self.shut_down_tasks_and_channels();
                Err("all peer set background tasks have exited".into())
            }
        }
    }

    /// Shut down:
    /// - services by dropping the service lists
    /// - background tasks via their join handles or cancel handles
    /// - channels by closing the channel
    fn shut_down_tasks_and_channels(&mut self) {
        // Drop services and cancel their background tasks.
        self.ready_services = HashMap::new();

        for (_peer_key, handle) in self.cancel_handles.drain() {
            let _ = handle.send(CancelClientWork);
        }
        self.unready_services = FuturesUnordered::new();

        // Close the MorePeers channel for all senders,
        // so we don't add more peers to a shut down peer set.
        self.demand_signal.close_channel();

        // Shut down background tasks.
        self.handle_rx.close();
        self.receive_tasks_if_needed();
        for guard in self.guards.iter() {
            guard.abort();
        }

        // TODO: implement graceful shutdown for InventoryRegistry (#1678)
    }

    /// Check busy peer services for request completion or errors.
    ///
    /// Move newly ready services to the ready list if they are for peers with supported protocol
    /// versions, otherwise they are dropped. Also drop failed services.
    fn poll_unready(&mut self, cx: &mut Context<'_>) {
        loop {
            match Pin::new(&mut self.unready_services).poll_next(cx) {
                // No unready service changes, or empty unready services
                Poll::Pending | Poll::Ready(None) => return,

                // Unready -> Ready
                Poll::Ready(Some(Ok((key, svc)))) => {
                    tracing::trace!(?key, "service became ready");
                    let cancel = self.cancel_handles.remove(&key);
                    assert!(cancel.is_some(), "missing cancel handle");
                }

                // Unready -> Canceled
                Poll::Ready(Some(Err((key, UnreadyError::Canceled)))) => {
                    // A service be canceled because we've connected to the same service twice.
                    // In that case, there is a cancel handle for the peer address,
                    // but it belongs to the service for the newer connection.
                    tracing::trace!(
                        ?key,
                        duplicate_connection = self.cancel_handles.contains_key(&key),
                        "service was canceled, dropping service"
                    );
                }
                Poll::Ready(Some(Err((key, UnreadyError::CancelHandleDropped(_))))) => {
                    // Similarly, services with dropped cancel handes can have duplicates.
                    tracing::trace!(
                        ?key,
                        duplicate_connection = self.cancel_handles.contains_key(&key),
                        "cancel handle was dropped, dropping service"
                    );
                }

                // Unready -> Errored
                Poll::Ready(Some(Err((key, UnreadyError::Inner(error))))) => {
                    tracing::debug!(%error, "service failed while unready, dropping service");

                    let cancel = self.cancel_handles.remove(&key);
                    assert!(cancel.is_some(), "missing cancel handle");
                }
            }
        }
    }

    /// Checks for newly inserted or removed services.
    ///
    /// Puts inserted services in the unready list.
    /// Drops removed services, after cancelling any pending requests.
    fn poll_discover(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), BoxError>> {
        use futures::ready;
        loop {
            match ready!(Pin::new(&mut self.discover).poll_discover(cx))
                .ok_or("discovery stream closed")?
                .map_err(Into::into)?
            {
                Change::Remove(key) => {
                    tracing::trace!(?key, "got Change::Remove from Discover");
                    self.remove(&key);
                }
                Change::Insert(key, svc) => {
                    // We add peers as unready, so that we:
                    // - always do the same checks on every ready peer, and
                    // - check for any errors that happened right after the handshake
                    tracing::trace!(?key, "got Change::Insert from Discover");
                    self.remove(&key);
                    self.push_unready(key, svc);
                }
            }
        }
    }

    /// Calls the poll functions used at the start of all `poll_ready`s
    pub fn poll_all(&mut self, cx: &mut Context<'_>) -> Result<(), BoxError> {
        self.poll_background_errors(cx)?;

        // Update peer statuses
        let _ = self.poll_discover(cx)?;
        self.poll_unready(cx);
        Ok(())
    }

    pub fn poll_ready(&mut self, cx: &mut Context<'_>) {
        let mut ready_services = HashMap::with_capacity(self.ready_services.len());
        let mut pending_services = vec![];
        for (key, mut svc) in self.ready_services.drain() {
            match svc.poll_ready(cx) {
                Poll::Pending => {
                    pending_services.push((key, svc));
                }
                Poll::Ready(Ok(())) => {
                    ready_services.insert(key, svc);
                }
                Poll::Ready(Err(e)) => {
                    tracing::trace!("Peer poll_ready returned error: {}", e);
                    // peer svc will get dropped at the start of next loop
                }
            }
        }
        for (key, svc) in pending_services {
            self.push_unready(key, svc);
        }
        self.ready_services = ready_services;
    }

    pub fn proportion_ready(&self) -> f64 {
        let total_services = self.ready_services.len() + self.unready_services.len();

        if total_services == 0 {
            return 1.0;
        }

        self.ready_services.len() as f64 / total_services as f64
    }

    /// Takes a ready service by key.
    pub fn take_ready_service(&mut self, key: &D::Key) -> Option<D::Service> {
        if let Some(svc) = self.ready_services.remove(key) {
            assert!(
                !self.cancel_handles.contains_key(key),
                "cancel handles are only used for unready service work"
            );

            Some(svc)
        } else {
            None
        }
    }

    /// Remove the service corresponding to `key` from the peer set.
    ///
    /// Drops the service, cancelling any pending request or response to that peer.
    /// If the peer does not exist, does nothing.
    fn remove(&mut self, key: &D::Key) {
        if let Some(ready_service) = self.take_ready_service(key) {
            // A ready service has no work to cancel, so just drop it.
            std::mem::drop(ready_service);
        } else if let Some(handle) = self.cancel_handles.remove(key) {
            // Cancel the work, implicitly dropping the cancel handle.
            // The service future returns a `Canceled` error,
            // making `poll_unready` drop the service.
            let _ = handle.send(CancelClientWork);
        }
    }

    /// Adds a busy service to the unready list if it's for a peer with a supported version,
    /// and adds a cancel handle for the service's current request.
    ///
    /// If the service is for a connection to an outdated peer, the request is cancelled and the
    /// service is dropped.
    pub fn push_unready(&mut self, key: D::Key, svc: D::Service) {
        let (tx, rx) = oneshot::channel();

        self.unready_services.push(UnreadyService {
            key: Some(key),
            service: Some(svc),
            cancel: rx,
            _req: PhantomData,
        });

        self.cancel_handles.insert(key, tx);
    }

    pub fn preselect_p2c_peer_with_full_block(&self, block_height: u64) -> Option<D::Key> {
        self.select_p2c_peer_from_list(
            &self
                .ready_services
                .iter()
                .filter_map(|(key, serv)| {
                    if serv.has_full_block(block_height) {
                        Some(key)
                    } else {
                        None
                    }
                })
                .collect(),
        )
    }

    /// Performs P2C on `self.ready_services` to randomly select a less-loaded ready service.
    pub fn preselect_p2c_peer(&self) -> Option<D::Key> {
        self.select_p2c_peer_from_list(&self.ready_services.keys().collect())
    }

    /// Accesses a ready endpoint by `key` and returns its current load.
    ///
    /// Returns `None` if the service is not in the ready service list.
    fn query_load(&self, key: &D::Key) -> Option<<D::Service as Load>::Metric> {
        let svc = self.ready_services.get(key);
        svc.map(|svc| svc.load())
    }

    // Performs P2C on `ready_service_list` to randomly select a less-loaded ready service.
    #[allow(clippy::unwrap_in_result)]
    pub fn select_p2c_peer_from_list(
        &self,
        ready_service_list: &HashSet<&D::Key>,
    ) -> Option<D::Key> {
        match ready_service_list.len() {
            0 => None,
            1 => Some(
                **ready_service_list
                    .iter()
                    .next()
                    .expect("just checked there is one service"),
            ),
            len => {
                // If there are only 2 peers, randomise their order.
                // Otherwise, choose 2 random peers in a random order.
                let (a, b) = {
                    let idxs = rand::seq::index::sample(&mut rand::thread_rng(), len, 2);
                    let a = idxs.index(0);
                    let b = idxs.index(1);

                    let a = **ready_service_list
                        .iter()
                        .nth(a)
                        .expect("sample returns valid indexes");
                    let b = **ready_service_list
                        .iter()
                        .nth(b)
                        .expect("sample returns valid indexes");

                    (a, b)
                };

                let a_load = self.query_load(&a).expect("supplied services are ready");
                let b_load = self.query_load(&b).expect("supplied services are ready");

                let selected = if a_load <= b_load { a } else { b };

                tracing::trace!(
                    a.key = ?a,
                    a.load = ?a_load,
                    b.key = ?b,
                    b.load = ?b_load,
                    selected = ?selected,
                    ?len,
                    "selected service by p2c"
                );

                Some(selected)
            }
        }
    }

    pub fn all_ready(&mut self) -> &mut HashMap<PeerID, LoadTrackedClient> {
        &mut self.ready_services
    }

    pub fn push_all_unready(&mut self) {
        let all_ready: Vec<(_, _)> = self.ready_services.drain().collect();
        for (key, svc) in all_ready {
            self.push_unready(key, svc)
        }
    }
    pub fn demand_more_peers(&mut self) {
        let _ = self.demand_signal.try_send(MorePeers);
    }
}
