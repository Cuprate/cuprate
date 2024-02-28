use monero_pruning::{PruningError, PruningSeed};
use monero_wire::{NetZone, NetworkAddress, PeerListEntryBase};

use crate::{
    client::InternalPeerID, handles::ConnectionHandle, NetZoneAddress, NetworkAddressIncorrectZone,
    NetworkZone,
};

pub enum CoreSyncDataRequest {
    Ours,
    HandleIncoming(monero_wire::CoreSyncData),
}

pub enum CoreSyncDataResponse {
    Ours(monero_wire::CoreSyncData),
    Ok,
}

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

impl<A: NetZoneAddress> From<ZoneSpecificPeerListEntryBase<A>> for monero_wire::PeerListEntryBase {
    fn from(value: ZoneSpecificPeerListEntryBase<A>) -> Self {
        Self {
            adr: value.adr.into(),
            id: value.id,
            last_seen: value.last_seen,
            pruning_seed: value.pruning_seed.into(),
            rpc_port: value.rpc_port,
            rpc_credits_per_hash: value.rpc_credits_per_hash,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PeerListConversionError {
    #[error("Address is in incorrect zone")]
    Address(#[from] NetworkAddressIncorrectZone),
    #[error("Pruning seed error: {0}")]
    PruningSeed(#[from] PruningError),
}

impl<A: NetZoneAddress> TryFrom<monero_wire::PeerListEntryBase>
    for ZoneSpecificPeerListEntryBase<A>
{
    type Error = PeerListConversionError;

    fn try_from(value: PeerListEntryBase) -> Result<Self, Self::Error> {
        Ok(Self {
            adr: value.adr.try_into()?,
            id: value.id,
            last_seen: value.last_seen,
            pruning_seed: PruningSeed::try_from(value.pruning_seed)?,
            rpc_port: value.rpc_port,
            rpc_credits_per_hash: value.rpc_credits_per_hash,
        })
    }
}

pub enum AddressBookRequest<Z: NetworkZone> {
    NewConnection {
        addr: Option<Z::Addr>,
        internal_peer_id: InternalPeerID<Z::Addr>,
        handle: ConnectionHandle,
        id: u64,
        pruning_seed: PruningSeed,
        /// The peers port.
        rpc_port: u16,
        /// The peers rpc credits per hash
        rpc_credits_per_hash: u32,
    },
    /// Bans a peer for the specified duration. This request
    /// will send disconnect signals to all peers with the same
    /// address.
    BanPeer(Z::Addr, std::time::Duration),
    IncomingPeerList(Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>),
    /// Gets a random white peer from the peer list. If height is specified
    /// then the peer list should retrieve a peer that should have a full
    /// block at that height according to it's pruning seed
    GetRandomWhitePeer {
        height: Option<u64>,
    },
    /// Gets a random gray peer from the peer list. If height is specified
    /// then the peer list should retrieve a peer that should have a full
    /// block at that height according to it's pruning seed
    GetRandomGrayPeer {
        height: Option<u64>,
    },
    GetWhitePeers(usize),
}

pub enum AddressBookResponse<Z: NetworkZone> {
    Ok,
    Peer(ZoneSpecificPeerListEntryBase<Z::Addr>),
    Peers(Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>),
}
