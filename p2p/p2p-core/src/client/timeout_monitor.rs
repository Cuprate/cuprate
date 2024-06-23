//! Timeout Monitor
//!
//! This module holds the task that sends periodic [TimedSync](PeerRequest::TimedSync) requests to a peer to make
//! sure the connection is still active.
use std::sync::Arc;

use futures::channel::oneshot;
use tokio::{
    sync::{mpsc, Semaphore},
    time::{interval, MissedTickBehavior},
};
use tower::ServiceExt;
use tracing::instrument;

use cuprate_wire::admin::TimedSyncRequest;

use crate::{
    client::{connection::ConnectionTaskRequest, InternalPeerID},
    constants::{MAX_PEERS_IN_PEER_LIST_MESSAGE, TIMEOUT_INTERVAL},
    handles::ConnectionHandle,
    services::{AddressBookRequest, CoreSyncDataRequest, CoreSyncDataResponse, PeerSyncRequest},
    AddressBook, CoreSyncSvc, NetworkZone, PeerRequest, PeerResponse, PeerSyncSvc,
};

/// The timeout monitor task, this task will send periodic timed sync requests to the peer to make sure it is still active.
#[instrument(
    name = "timeout_monitor",
    level = "debug",
    fields(addr = %id),
    skip_all,
)]
pub async fn connection_timeout_monitor_task<N: NetworkZone, AdrBook, CSync, PSync>(
    id: InternalPeerID<N::Addr>,
    handle: ConnectionHandle,

    connection_tx: mpsc::Sender<ConnectionTaskRequest>,
    semaphore: Arc<Semaphore>,

    mut address_book_svc: AdrBook,
    mut core_sync_svc: CSync,
    mut peer_core_sync_svc: PSync,
) -> Result<(), tower::BoxError>
where
    AdrBook: AddressBook<N>,
    CSync: CoreSyncSvc,
    PSync: PeerSyncSvc<N>,
{
    // Instead of tracking the time from last message from the peer and sending a timed sync if this value is too high,
    // we just send a timed sync every [TIMEOUT_INTERVAL] seconds.
    let mut interval = interval(TIMEOUT_INTERVAL);

    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    // The first tick ticks instantly.
    interval.tick().await;

    loop {
        interval.tick().await;

        tracing::trace!("timeout monitor tick.");

        if connection_tx.is_closed() {
            tracing::debug!("Closing timeout monitor, connection disconnected.");
            return Ok(());
        }

        let Ok(permit) = semaphore.clone().try_acquire_owned() else {
            // If we can't get a permit the connection is currently waiting for a response, so no need to
            // do a timed sync.
            continue;
        };

        let ping_span = tracing::debug_span!("timed_sync");

        // get our core sync data
        tracing::trace!(parent: &ping_span, "Attempting to get our core sync data");
        let CoreSyncDataResponse(core_sync_data) = core_sync_svc
            .ready()
            .await?
            .call(CoreSyncDataRequest)
            .await?;

        let (tx, rx) = oneshot::channel();

        // TODO: Instead of always sending timed syncs, send pings if we have a full peer list.

        tracing::debug!(parent: &ping_span, "Sending timed sync to peer");
        connection_tx
            .send(ConnectionTaskRequest {
                request: PeerRequest::TimedSync(TimedSyncRequest {
                    payload_data: core_sync_data,
                }),
                response_channel: tx,
                permit: Some(permit),
            })
            .await?;

        let PeerResponse::TimedSync(timed_sync) = rx.await?? else {
            panic!("Connection task returned wrong response!");
        };

        tracing::debug!(
            parent: &ping_span,
            "Received timed sync response, incoming peer list len: {}",
            timed_sync.local_peerlist_new.len()
        );

        if timed_sync.local_peerlist_new.len() > MAX_PEERS_IN_PEER_LIST_MESSAGE {
            return Err("Peer sent too many peers in peer list".into());
        }

        // Tell our address book about the new peers.
        address_book_svc
            .ready()
            .await?
            .call(AddressBookRequest::IncomingPeerList(
                timed_sync
                    .local_peerlist_new
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            ))
            .await?;

        // Tell the peer sync service about the peers core sync data
        peer_core_sync_svc
            .ready()
            .await?
            .call(PeerSyncRequest::IncomingCoreSyncData(
                id,
                handle.clone(),
                timed_sync.payload_data,
            ))
            .await?;
    }
}
