use cuprate_pruning::{PruningError, PruningSeed};
use cuprate_wire::{CoreSyncData, PeerListEntryBase};

use crate::{
    client::InternalPeerID, handles::ConnectionHandle, NetZoneAddress, NetworkAddressIncorrectZone,
    NetworkZone,
};

/// A request to the service that keeps track of peers sync states.
pub enum PeerSyncRequest<N: NetworkZone> {
    /// Request some peers to sync from.
    ///
    /// This takes in the current cumulative difficulty of our chain and will return peers that
    /// claim to have a higher cumulative difficulty.
    PeersToSyncFrom {
        current_cumulative_difficulty: u128,
        block_needed: Option<usize>,
    },
    /// Add/update a peer's core sync data.
    IncomingCoreSyncData(InternalPeerID<N::Addr>, ConnectionHandle, CoreSyncData),
}

/// A response from the service that keeps track of peers sync states.
pub enum PeerSyncResponse<N: NetworkZone> {
    /// The return value of [`PeerSyncRequest::PeersToSyncFrom`].
    PeersToSyncFrom(Vec<InternalPeerID<N::Addr>>),
    /// A generic ok response.
    Ok,
}

/// A request to the core sync service for our node's [`CoreSyncData`].
pub struct CoreSyncDataRequest;

/// A response from the core sync service containing our [`CoreSyncData`].
pub struct CoreSyncDataResponse(pub CoreSyncData);

/// A [`NetworkZone`] specific [`PeerListEntryBase`].
///
/// Using this type instead of [`PeerListEntryBase`] in the address book makes
/// usage easier for the rest of the P2P code as we can guarantee only the correct addresses will be stored and returned.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct ZoneSpecificPeerListEntryBase<A: NetZoneAddress> {
    pub adr: A,
    pub id: u64,
    pub last_seen: i64,
    pub pruning_seed: PruningSeed,
    pub rpc_port: u16,
    pub rpc_credits_per_hash: u32,
}

impl<A: NetZoneAddress> From<ZoneSpecificPeerListEntryBase<A>> for cuprate_wire::PeerListEntryBase {
    fn from(value: ZoneSpecificPeerListEntryBase<A>) -> Self {
        Self {
            adr: value.adr.into(),
            id: value.id,
            last_seen: value.last_seen,
            pruning_seed: value.pruning_seed.compress(),
            rpc_port: value.rpc_port,
            rpc_credits_per_hash: value.rpc_credits_per_hash,
        }
    }
}

/// An error converting a [`PeerListEntryBase`] into a [`ZoneSpecificPeerListEntryBase`].
#[derive(Debug, thiserror::Error)]
pub enum PeerListConversionError {
    #[error("Address is in incorrect zone")]
    Address(#[from] NetworkAddressIncorrectZone),
    #[error("Pruning seed error: {0}")]
    PruningSeed(#[from] PruningError),
}

impl<A: NetZoneAddress> TryFrom<cuprate_wire::PeerListEntryBase>
    for ZoneSpecificPeerListEntryBase<A>
{
    type Error = PeerListConversionError;

    fn try_from(value: PeerListEntryBase) -> Result<Self, Self::Error> {
        Ok(Self {
            adr: value.adr.try_into()?,
            id: value.id,
            last_seen: value.last_seen,
            pruning_seed: PruningSeed::decompress_p2p_rules(value.pruning_seed)?,
            rpc_port: value.rpc_port,
            rpc_credits_per_hash: value.rpc_credits_per_hash,
        })
    }
}

/// A request to the address book service.
pub enum AddressBookRequest<Z: NetworkZone> {
    /// Tells the address book that we have connected or received a connection from a peer.
    NewConnection {
        /// The [`InternalPeerID`] of this connection.
        internal_peer_id: InternalPeerID<Z::Addr>,
        /// The public address of the peer, if this peer has a reachable public address.
        public_address: Option<Z::Addr>,
        /// The [`ConnectionHandle`] to this peer.
        handle: ConnectionHandle,
        /// An ID the peer assigned itself.
        id: u64,
        /// The peers [`PruningSeed`].
        pruning_seed: PruningSeed,
        /// The peers rpc port.
        rpc_port: u16,
        /// The peers rpc credits per hash
        rpc_credits_per_hash: u32,
    },
    /// Tells the address book about a peer list received from a peer.
    IncomingPeerList(Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>),
    /// Takes a random white peer from the peer list. If height is specified
    /// then the peer list should retrieve a peer that should have a full
    /// block at that height according to it's pruning seed
    TakeRandomWhitePeer { height: Option<usize> },
    /// Takes a random gray peer from the peer list. If height is specified
    /// then the peer list should retrieve a peer that should have a full
    /// block at that height according to it's pruning seed
    TakeRandomGrayPeer { height: Option<usize> },
    /// Takes a random peer from the peer list. If height is specified
    /// then the peer list should retrieve a peer that should have a full
    /// block at that height according to it's pruning seed.
    ///
    /// The address book will look in the white peer list first, then the gray
    /// one if no peer is found.
    TakeRandomPeer { height: Option<usize> },
    /// Gets the specified number of white peers, or less if we don't have enough.
    GetWhitePeers(usize),
    /// Checks if the given peer is banned.
    IsPeerBanned(Z::Addr),
}

/// A response from the address book service.
pub enum AddressBookResponse<Z: NetworkZone> {
    Ok,
    Peer(ZoneSpecificPeerListEntryBase<Z::Addr>),
    Peers(Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>),
    /// Contains `true` if the peer is banned.
    IsPeerBanned(bool),
}
