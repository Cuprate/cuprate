//! [`From`] implementations from other crate's types into [`crate`] types.
//!
//! Only non-crate types are imported, all crate types use `crate::`.

use std::{
    net::{SocketAddr, SocketAddrV4},
    time::Duration,
};

use cuprate_helper::{fmt::HexPrefix, map::ipv4_from_u32};
use cuprate_hex::{Hex, HexVec};
use cuprate_p2p_core::{
    types::{ConnectionId, ConnectionInfo, SetBan, Span},
    NetZoneAddress,
};
use cuprate_types::rpc::{BlockHeader, ChainInfo, HistogramEntry, SpentKeyImageInfo, TxInfo};

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
            pow_hash: x.pow_hash.map_or_else(HexVec::new, |a| HexVec(a.into())),
            prev_hash: Hex(x.prev_hash),
            reward: x.reward,
            timestamp: x.timestamp,
            // FIXME: if we made a type that automatically did `hex_prefix_u128`,
            //  we wouldn't need `crate::misc::BlockHeader`.
            wide_cumulative_difficulty: (x.cumulative_difficulty, x.cumulative_difficulty_top64)
                .hex_prefix(),
            wide_difficulty: (x.difficulty, x.difficulty_top64).hex_prefix(),
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

impl From<ChainInfo> for crate::misc::ChainInfo {
    fn from(x: ChainInfo) -> Self {
        Self {
            block_hash: Hex(x.block_hash),
            block_hashes: x.block_hashes.into_iter().map(Hex).collect(),
            difficulty_top64: x.difficulty_top64,
            difficulty: x.difficulty,
            height: x.height,
            length: x.length,
            main_chain_parent_block: Hex(x.main_chain_parent_block),
            wide_difficulty: (x.difficulty, x.difficulty_top64).hex_prefix(),
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
            tx_blob: HexVec(x.tx_blob),
            tx_json: x.tx_json,
            weight: x.weight,
        }
    }
}

impl From<SpentKeyImageInfo> for crate::misc::SpentKeyImageInfo {
    fn from(x: SpentKeyImageInfo) -> Self {
        Self {
            id_hash: Hex(x.id_hash),
            txs_hashes: x.txs_hashes.into_iter().map(Hex).collect(),
        }
    }
}

impl From<crate::misc::OutKeyBin> for crate::misc::OutKey {
    fn from(x: crate::misc::OutKeyBin) -> Self {
        Self {
            key: Hex(x.key),
            mask: Hex(x.mask),
            unlocked: x.unlocked,
            height: x.height,
            txid: if x.txid == [0; 32] {
                HexVec::new()
            } else {
                HexVec::from(x.txid)
            },
        }
    }
}
