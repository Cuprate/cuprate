use crate::sealed::{BorshNetZoneAddress, BorshNetworkZone};
use cuprate_p2p_core::{ClearNet, NetZoneAddress, NetworkZone};
use std::collections::HashMap;
use std::fmt::Display;
use std::net::{AddrParseError, IpAddr, SocketAddr};
use std::num::ParseIntError;
use std::str::FromStr;
use std::task::Poll;
use std::{task::Context, time::Duration};
use tokio::time::Instant;
use tokio_util::time::DelayQueue;

pub trait BanList<A: BorshNetZoneAddress>: Default {
    type DiskFmt: borsh::BorshDeserialize + borsh::BorshSerialize;

    fn is_banned(&self, addr: &A) -> bool;

    fn ban(&mut self, host: A::BanID, duration: Duration);

    fn unbanned_instant(&self, addr: &A) -> Option<Instant>;

    fn poll_bans(&mut self, cx: &mut Context<'_>);

    fn append(&mut self, other: &mut Self);

    fn disk_fmt(&self) -> Self::DiskFmt;

    fn from_disk_fmt(disk_fmt: Self::DiskFmt) -> Self;
}

pub struct GenericBanList<A: BorshNetZoneAddress> {
    banned_peers: HashMap<A::BanID, Instant>,
    banned_peers_queue: DelayQueue<A::BanID>,
}

impl<A: BorshNetZoneAddress> Default for GenericBanList<A> {
    fn default() -> Self {
        Self {
            banned_peers: HashMap::new(),
            banned_peers_queue: DelayQueue::new(),
        }
    }
}

impl<A: BorshNetZoneAddress> BanList<A> for GenericBanList<A> {
    type DiskFmt = Vec<(A::BanID, u64)>;

    fn is_banned(&self, addr: &A) -> bool {
        self.banned_peers.contains_key(&addr.ban_id())
    }

    fn ban(&mut self, host: A::BanID, duration: Duration) {
        let time = Instant::now() + duration;
        self.banned_peers_queue.insert_at(host, time);
        self.banned_peers.insert(host, time);
    }

    fn unbanned_instant(&self, addr: &A) -> Option<Instant> {
        self.banned_peers.get(&addr.ban_id()).copied()
    }

    fn poll_bans(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(ban_id)) = self.banned_peers_queue.poll_expired(cx) {
            tracing::info!("Host {:?} is unbanned, ban has expired.", ban_id.get_ref(),);
            self.banned_peers.remove(ban_id.get_ref());
        }
    }

    fn append(&mut self, other: &mut Self) {
        for (host, time) in other.banned_peers.drain() {
            self.banned_peers_queue.insert_at(host, time);
            self.banned_peers.insert(host, time);
        }
    }

    fn disk_fmt(&self) -> Self::DiskFmt {
        self.banned_peers
            .iter()
            .map(|(host, time)| {
                (
                    *host,
                    time.saturating_duration_since(Instant::now()).as_secs(),
                )
            })
            .collect()
    }

    fn from_disk_fmt(disk_fmt: Self::DiskFmt) -> Self {
        let mut ban_list = Self::default();
        for (addr, time) in disk_fmt {
            ban_list.ban(addr, Duration::from_secs(time));
        }
        ban_list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IpSubnet {
    ip: IpAddr,
    bits: u64,
}

impl Display for IpSubnet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.ip, self.bits)
    }
}

#[derive(Debug)]
pub enum IpSubnetParseError {
    InvalidIp(AddrParseError),
    InvalidBits(ParseIntError),
    InvalidSubnet,
}

impl Display for IpSubnetParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIp(e) => e.fmt(f),
            Self::InvalidBits(e) => e.fmt(f),
            Self::InvalidSubnet => write!(f, "Invalid subnet"),
        }
    }
}

impl FromStr for IpSubnet {
    type Err = IpSubnetParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('/');

        Ok(Self {
            ip: parts
                .next()
                .map(str::parse)
                .map_or(Err(IpSubnetParseError::InvalidSubnet), |r| {
                    r.map_err(IpSubnetParseError::InvalidIp)
                })?,
            bits: parts
                .next()
                .map(str::parse)
                .map_or(Err(IpSubnetParseError::InvalidSubnet), |r| {
                    r.map_err(IpSubnetParseError::InvalidBits)
                })?,
        })
    }
}

#[derive(Default)]
pub struct IpBanList {
    pub banned_subnets: Vec<IpSubnet>,
    pub generic_ban_list: GenericBanList<SocketAddr>,
}

impl BanList<SocketAddr> for IpBanList {
    type DiskFmt = <GenericBanList<SocketAddr> as BanList<SocketAddr>>::DiskFmt;

    fn is_banned(&self, addr: &SocketAddr) -> bool {
        for subnet in &self.banned_subnets {
            match (addr.ip(), subnet.ip) {
                (IpAddr::V4(left), IpAddr::V4(right)) => {
                    let mask = u32::MAX << (32 - subnet.bits);
                    if left.to_bits() & mask == right.to_bits() & mask {
                        return true;
                    }
                }
                (IpAddr::V6(left), IpAddr::V6(right)) => {
                    let mask = u128::MAX << (128 - subnet.bits);
                    if left.to_bits() & mask == right.to_bits() & mask {
                        return true;
                    }
                }
                _ => continue,
            }
        }

        self.generic_ban_list.is_banned(addr)
    }

    fn ban(&mut self, host: <SocketAddr as NetZoneAddress>::BanID, duration: Duration) {
        self.generic_ban_list.ban(host, duration);
    }

    fn unbanned_instant(&self, addr: &SocketAddr) -> Option<Instant> {
        self.generic_ban_list.unbanned_instant(addr)
    }

    fn poll_bans(&mut self, cx: &mut Context<'_>) {
        self.generic_ban_list.poll_bans(cx);
    }

    fn append(&mut self, other: &mut Self) {
        self.banned_subnets.append(&mut other.banned_subnets);
        self.generic_ban_list.append(&mut other.generic_ban_list)
    }

    fn disk_fmt(&self) -> Self::DiskFmt {
        self.generic_ban_list.disk_fmt()
    }

    fn from_disk_fmt(disk_fmt: Self::DiskFmt) -> Self {
        let generic_ban_list = GenericBanList::<SocketAddr>::from_disk_fmt(disk_fmt);

        Self {
            banned_subnets: Vec::new(),
            generic_ban_list,
        }
    }
}
