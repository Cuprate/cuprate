//! General data structures.

use std::time::{Duration, Instant};

use cuprate_pruning::PruningSeed;

use crate::NetZoneAddress;

/// Data within [`crate::services::AddressBookRequest::SetBan`].
pub struct SetBan<A: NetZoneAddress> {
    /// Address of the peer.
    pub address: A,
    /// - If [`Some`], how long this peer should be banned for
    /// - If [`None`], the peer will be unbanned
    pub ban: Option<Duration>,
}

/// Data within [`crate::services::AddressBookResponse::GetBans`].
pub struct BanState<A: NetZoneAddress> {
    /// Address of the peer.
    pub address: A,
    /// - If [`Some`], the peer is banned until this [`Instant`]
    /// - If [`None`], the peer is not currently banned
    pub unban_instant: Option<Instant>,
}

/// An enumeration of address types.
///
/// Used [`ConnectionInfo::address_type`].
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AddressType {
    #[default]
    Invalid,
    Ipv4,
    Ipv6,
    I2p,
    Tor,
}

impl AddressType {
    /// Convert [`Self`] to a [`u8`].
    ///
    /// ```rust
    /// use cuprate_p2p_core::types::AddressType as A;
    ///
    /// assert_eq!(A::Invalid.to_u8(), 0);
    /// assert_eq!(A::Ipv4.to_u8(), 1);
    /// assert_eq!(A::Ipv6.to_u8(), 2);
    /// assert_eq!(A::I2p.to_u8(), 3);
    /// assert_eq!(A::Tor.to_u8(), 4);
    /// ```
    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    /// Convert a [`u8`] to a [`Self`].
    ///
    /// # Errors
    /// This returns [`None`] if `u > 4`.
    ///
    /// ```rust
    /// use cuprate_p2p_core::types::AddressType as A;
    ///
    /// assert_eq!(A::from_u8(0), Some(A::Invalid));
    /// assert_eq!(A::from_u8(1), Some(A::Ipv4));
    /// assert_eq!(A::from_u8(2), Some(A::Ipv6));
    /// assert_eq!(A::from_u8(3), Some(A::I2p));
    /// assert_eq!(A::from_u8(4), Some(A::Tor));
    /// assert_eq!(A::from_u8(5), None);
    /// ```
    pub const fn from_u8(u: u8) -> Option<Self> {
        Some(match u {
            0 => Self::Invalid,
            1 => Self::Ipv4,
            2 => Self::Ipv6,
            3 => Self::I2p,
            4 => Self::Tor,
            _ => return None,
        })
    }
}

// TODO: reduce fields and map to RPC type.
//
/// Data within [`crate::services::AddressBookResponse::ConnectionInfo`].
pub struct ConnectionInfo<A: NetZoneAddress> {
    pub address: A,
    pub address_type: AddressType,
    pub avg_download: u64,
    pub avg_upload: u64,
    pub connection_id: u64, // TODO: boost::uuids::uuid
    pub current_download: u64,
    pub current_upload: u64,
    pub height: u64,
    pub host: String,
    pub incoming: bool,
    pub ip: String,
    pub live_time: u64,
    pub localhost: bool,
    pub local_ip: bool,
    pub peer_id: String,
    pub port: String,
    pub pruning_seed: PruningSeed,
    pub recv_count: u64,
    pub recv_idle_time: u64,
    pub rpc_credits_per_hash: u32,
    pub rpc_port: u16,
    pub send_count: u64,
    pub send_idle_time: u64,
    pub state: String, // TODO: what type is this?
    pub support_flags: u32,
}

/// Used in RPC's `sync_info`.
///
/// Data within [`crate::services::AddressBookResponse::Spans`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span<A: NetZoneAddress> {
    pub connection_id: u64, // TODO: boost::uuids::uuid
    pub nblocks: u64,
    pub rate: u32,
    pub remote_address: A,
    pub size: u64,
    pub speed: u32,
    pub start_block_height: u64,
}
