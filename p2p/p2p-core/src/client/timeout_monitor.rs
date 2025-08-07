//! Timeout Monitor
//!
//! This module holds the task that sends periodic [`TimedSync`](PeerRequest::TimedSync) requests to a peer to make
//! sure the connection is still active.
use std::sync::Arc;

use futures::channel::oneshot;
use tokio::{
    sync::{mpsc, Semaphore},
    time::{interval, MissedTickBehavior},
};
use tower::ServiceExt;
use tracing::instrument;

use cuprate_wire::{admin::TimedSyncRequest, AdminRequestMessage, AdminResponseMessage};

use crate::{
    client::{connection::ConnectionTaskRequest, PeerInformation},
    constants::{MAX_PEERS_IN_PEER_LIST_MESSAGE, TIMEOUT_INTERVAL},
    services::{AddressBookRequest, CoreSyncDataRequest, CoreSyncDataResponse},
    AddressBook, CoreSyncSvc, NetworkZone, PeerRequest, PeerResponse,
};

/// The timeout monitor task, this task will send periodic timed sync requests to the peer to make sure it is still active.
#[instrument(
    name = "timeout_monitor",
    level = "debug",
    fields(addr = %peer_information.id),
    skip_all,
)]
pub(super) async fn connection_timeout_monitor_task<N: NetworkZone, AdrBook, CSync>(
    peer_information: PeerInformation<N::Addr>,

    connection_tx: mpsc::Sender<ConnectionTaskRequest>,
    semaphore: Arc<Semaphore>,

    mut address_book_svc: AdrBook,
    mut core_sync_svc: CSync,
) -> Result<(), tower::BoxError>
where
    AdrBook: AddressBook<N>,
    CSync: CoreSyncSvc,
{
    let connection_tx_weak = connection_tx.downgrade();
    drop(connection_tx);

    // Instead of tracking the time from last message from the peer and sending a timed sync if this value is too high,
    // we just send a timed sync every [TIMEOUT_INTERVAL] seconds.
    let mut interval = interval(TIMEOUT_INTERVAL);

    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    // The first tick ticks instantly.
    interval.tick().await;

    loop {
        tokio::select! {
            () = peer_information.handle.closed() => {
                tracing::debug!("Closing timeout monitor, connection disconnected.");
                return Ok(());
            }
            _ = interval.tick() => ()
        }

        tracing::trace!("timeout monitor tick.");

        let Some(connection_tx) = connection_tx_weak.upgrade() else {
            tracing::debug!("Closing timeout monitor, connection disconnected.");
            return Ok(());
        };

        let Ok(permit) = Arc::clone(&semaphore).try_acquire_owned() else {
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
                request: PeerRequest::Admin(AdminRequestMessage::TimedSync(TimedSyncRequest {
                    payload_data: core_sync_data,
                })),
                response_channel: tx,
                permit: Some(permit),
            })
            .await?;

        let PeerResponse::Admin(AdminResponseMessage::TimedSync(timed_sync)) = rx.await?? else {
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
                peer_information.id,
                timed_sync
                    .local_peerlist_new
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            ))
            .await?;

        *peer_information.core_sync_data.lock().unwrap() = timed_sync.payload_data;
    }
}
