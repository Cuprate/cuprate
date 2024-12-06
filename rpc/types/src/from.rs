//! [`From`] implementations from other crate's types into [`crate`] types.
//!
//! Only non-crate types are imported, all crate types use `crate::`.

#![allow(unused_variables, unreachable_code, reason = "TODO")]

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use cuprate_helper::map::combine_low_high_bits_to_u128;
use cuprate_p2p_core::{
    types::{BanState, ConnectionId, ConnectionInfo, SetBan, Span},
    ClearNet, NetZoneAddress, NetworkZone,
};
use cuprate_types::{
    hex::Hex,
    rpc::{
        AuxPow, BlockHeader, BlockOutputIndices, ChainInfo, GetBan, GetMinerDataTxBacklogEntry,
        GetOutputsOut, HardforkEntry, HistogramEntry, OutKey, OutKeyBin, OutputDistributionData,
        Peer, PublicNode, SpentKeyImageInfo, TxBacklogEntry, TxInfo, TxOutputIndices, TxpoolHisto,
        TxpoolStats,
    },
};

/// <https://architecture.cuprate.org/oddities/le-ipv4.html>
const fn ipv4_from_u32(ip: u32) -> Ipv4Addr {
    let [a, b, c, d] = ip.to_le_bytes();
    Ipv4Addr::new(a, b, c, d)
}

/// Format two [`u64`]'s as a [`u128`] as a hexadecimal string prefixed with `0x`.
fn hex_prefix_u128(low: u64, high: u64) -> String {
    format!("{:#x}", combine_low_high_bits_to_u128(low, high))
}

impl From<BlockHeader> for crate::misc::BlockHeader {
    fn from(x: BlockHeader) -> Self {
        Self {
            block_size: x.block_weight,
            block_weight: x.block_weight,
            cumulative_difficulty_top64: x.cumulative_difficulty_top64,
            cumulative_difficulty: x.cumulative_difficulty,
            depth: x.depth,
            difficulty_top64: x.difficulty_top64,
            difficulty: x.difficulty,
            hash: Hex(x.hash),
            height: x.height,
            long_term_weight: x.long_term_weight,
            major_version: x.major_version,
            miner_tx_hash: Hex(x.miner_tx_hash),
            minor_version: x.minor_version,
            nonce: x.nonce,
            num_txes: x.num_txes,
            orphan_status: x.orphan_status,
            pow_hash: Hex(x.pow_hash),
            prev_hash: Hex(x.prev_hash),
            reward: x.reward,
            timestamp: x.timestamp,
            // FIXME: if we made a type that automatically did `hex_prefix_u128`,
            //  we wouldn't need `crate::misc::BlockHeader`.
            wide_cumulative_difficulty: hex_prefix_u128(
                x.cumulative_difficulty,
                x.cumulative_difficulty_top64,
            ),
            wide_difficulty: hex_prefix_u128(x.difficulty, x.difficulty_top64),
        }
    }
}

impl<A: NetZoneAddress> From<ConnectionInfo<A>> for crate::misc::ConnectionInfo {
    fn from(x: ConnectionInfo<A>) -> Self {
        let (ip, port) = match x.socket_addr {
            Some(socket) => (socket.ip().to_string(), socket.port().to_string()),
            None => (String::new(), String::new()),
        };

        Self {
            address: x.address.to_string(),
            address_type: x.address_type,
            avg_download: x.avg_download,
            avg_upload: x.avg_upload,
            connection_id: String::from(ConnectionId::DEFAULT_STR),
            current_download: x.current_download,
            current_upload: x.current_upload,
            height: x.height,
            host: x.host,
            incoming: x.incoming,
            ip,
            live_time: x.live_time,
            localhost: x.localhost,
            local_ip: x.local_ip,
            peer_id: hex::encode(x.peer_id.to_ne_bytes()),
            port,
            pruning_seed: x.pruning_seed.compress(),
            recv_count: x.recv_count,
            recv_idle_time: x.recv_idle_time,
            rpc_credits_per_hash: x.rpc_credits_per_hash,
            rpc_port: x.rpc_port,
            send_count: x.send_count,
            send_idle_time: x.send_idle_time,
            state: x.state,
            support_flags: x.support_flags,
        }
    }
}

// TODO: support non-clearnet addresses.
impl From<crate::misc::SetBan> for SetBan<SocketAddr> {
    fn from(x: crate::misc::SetBan) -> Self {
        let address = SocketAddr::V4(SocketAddrV4::new(ipv4_from_u32(x.ip), 0));

        let ban = if x.ban {
            Some(Duration::from_secs(x.seconds.into()))
        } else {
            None
        };

        Self { address, ban }
    }
}

// TODO: do we need this type?
impl From<HistogramEntry> for crate::misc::HistogramEntry {
    fn from(x: HistogramEntry) -> Self {
        Self {
            amount: x.amount,
            total_instances: x.total_instances,
            unlocked_instances: x.unlocked_instances,
            recent_instances: x.recent_instances,
        }
    }
}

// impl From<HardforkEntry> for crate::misc::HardforkEntry {
//     fn from(x: HardforkEntry) -> Self {
//         Self {
//             height: x.height,
//             hf_version: x.hf_version,
//         }
//     }
// }

impl From<ChainInfo> for crate::misc::ChainInfo {
    fn from(x: ChainInfo) -> Self {
        Self {
            block_hash: Hex(x.block_hash),
            block_hashes: x.block_hashes.into_iter().map(hex::encode).collect(),
            difficulty_top64: x.difficulty_top64,
            difficulty: x.difficulty,
            height: x.height,
            length: x.length,
            main_chain_parent_block: Hex(x.main_chain_parent_block),
            wide_difficulty: hex_prefix_u128(x.difficulty, x.difficulty_top64),
        }
    }
}

// TODO: support non-clearnet addresses.
impl From<Span<SocketAddr>> for crate::misc::Span {
    fn from(x: Span<SocketAddr>) -> Self {
        Self {
            connection_id: String::from(ConnectionId::DEFAULT_STR),
            nblocks: x.nblocks,
            rate: x.rate,
            remote_address: x.remote_address.to_string(),
            size: x.size,
            speed: x.speed,
            start_block_height: x.start_block_height,
        }
    }
}

// impl From<OutputDistributionData> for crate::misc::OutputDistributionData {
//     fn from(x: OutputDistributionData) -> Self {
//         todo!();

//         // Self {
//         // 	distribution: Vec<u64>,
//         // 	start_height: u64,
//         // 	base: u64,
//         // }
//     }
// }

impl From<TxInfo> for crate::misc::TxInfo {
    fn from(x: TxInfo) -> Self {
        Self {
            blob_size: x.blob_size,
            do_not_relay: x.do_not_relay,
            double_spend_seen: x.double_spend_seen,
            fee: x.fee,
            id_hash: Hex(x.id_hash),
            kept_by_block: x.kept_by_block,
            last_failed_height: x.last_failed_height,
            last_failed_id_hash: Hex(x.last_failed_id_hash),
            last_relayed_time: x.last_relayed_time,
            max_used_block_height: x.max_used_block_height,
            max_used_block_id_hash: Hex(x.max_used_block_id_hash),
            receive_time: x.receive_time,
            relayed: x.relayed,
            tx_blob: hex::encode(x.tx_blob),
            tx_json: x.tx_json,
            weight: x.weight,
        }
    }
}

impl From<AuxPow> for crate::misc::AuxPow {
    fn from(x: AuxPow) -> Self {
        Self {
            id: Hex(x.id),
            hash: Hex(x.hash),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex() {
        assert_eq!(hex_prefix_u128(0, 0), "0x0");
        assert_eq!(hex_prefix_u128(0, u64::MAX), "0x0");
        assert_eq!(hex_prefix_u128(u64::MAX, 0), "0x0");
        assert_eq!(hex_prefix_u128(u64::MAX, u64::MAX), "0x0");
    }
}
