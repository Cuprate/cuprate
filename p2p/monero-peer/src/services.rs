use monero_wire::PeerListEntryBase;

use crate::{NetworkAddressIncorrectZone, NetworkZone};

pub enum CoreSyncDataRequest {
    Ours,
    HandleIncoming(monero_wire::CoreSyncData),
}

pub enum CoreSyncDataResponse {
    Ours(monero_wire::CoreSyncData),
    Ok,
}

pub struct ZoneSpecificPeerListEntryBase<Z: NetworkZone> {
    pub adr: Z::Addr,
    pub id: u64,
    pub last_seen: i64,
    pub pruning_seed: u32,
    pub rpc_port: u16,
    pub rpc_credits_per_hash: u32,
}

impl<Z: NetworkZone> From<ZoneSpecificPeerListEntryBase<Z>> for monero_wire::PeerListEntryBase {
    fn from(value: ZoneSpecificPeerListEntryBase<Z>) -> Self {
        Self {
            adr: value.adr.into(),
            id: value.id,
            last_seen: value.last_seen,
            pruning_seed: value.pruning_seed,
            rpc_port: value.rpc_port,
            rpc_credits_per_hash: value.rpc_credits_per_hash,
        }
    }
}

impl<Z: NetworkZone> TryFrom<monero_wire::PeerListEntryBase> for ZoneSpecificPeerListEntryBase<Z> {
    type Error = NetworkAddressIncorrectZone;

    fn try_from(value: PeerListEntryBase) -> Result<Self, Self::Error> {
        Ok(Self {
            adr: value.adr.try_into()?,
            id: value.id,
            last_seen: value.last_seen,
            pruning_seed: value.pruning_seed,
            rpc_port: value.rpc_port,
            rpc_credits_per_hash: value.rpc_credits_per_hash,
        })
    }
}

pub enum AddressBookRequest<Z: NetworkZone> {
    NewConnection(Z::Addr, ZoneSpecificPeerListEntryBase<Z>),
    IncomingPeerList(Vec<ZoneSpecificPeerListEntryBase<Z>>),
    GetPeers(usize),
}

pub enum AddressBookResponse<Z: NetworkZone> {
    Ok,
    Peers(Vec<ZoneSpecificPeerListEntryBase<Z>>),
}
