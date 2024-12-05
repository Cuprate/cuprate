//! [`From`] implementations from other crate's types into [`crate`] types.

#![allow(unused_variables, unreachable_code, reason = "TODO")]

use cuprate_types::rpc::{
    AuxPow, BlockHeader, BlockOutputIndices, ChainInfo, ConnectionInfo, GetBan,
    GetMinerDataTxBacklogEntry, GetOutputsOut, HardforkEntry, HistogramEntry, OutKey, OutKeyBin,
    OutputDistributionData, Peer, PublicNode, SetBan, Span, SpentKeyImageInfo, SyncInfoPeer,
    TxBacklogEntry, TxInfo, TxOutputIndices, TxpoolHisto, TxpoolStats,
};

impl From<BlockHeader> for crate::misc::BlockHeader {
    fn from(x: BlockHeader) -> Self {
        todo!();

        // Self {
        // 	block_size: u64,
        // 	block_weight: u64,
        // 	cumulative_difficulty_top64: u64,
        // 	cumulative_difficulty: u64,
        // 	depth: u64,
        // 	difficulty_top64: u64,
        // 	difficulty: u64,
        // 	hash: String,
        // 	height: u64,
        // 	long_term_weight: u64,
        // 	major_version: u8,
        // 	miner_tx_hash: String,
        // 	minor_version: u8,
        // 	nonce: u32,
        // 	num_txes: u64,
        // 	orphan_status: bool,
        // 	pow_hash: String,
        // 	prev_hash: String,
        // 	reward: u64,
        // 	timestamp: u64,
        // 	wide_cumulative_difficulty: String,
        // 	wide_difficulty: String,
        // }
    }
}

impl From<ConnectionInfo> for crate::misc::ConnectionInfo {
    fn from(x: ConnectionInfo) -> Self {
        todo!();

        // Self {
        // 	address: String,
        // 	address_type: AddressType,
        // 	avg_download: u64,
        // 	avg_upload: u64,
        // 	connection_id: String,
        // 	current_download: u64,
        // 	current_upload: u64,
        // 	height: u64,
        // 	host: String,
        // 	incoming: bool,
        // 	ip: String,
        // 	live_time: u64,
        // 	localhost: bool,
        // 	local_ip: bool,
        // 	peer_id: String,
        // 	port: String,
        // 	pruning_seed: u32,
        // 	recv_count: u64,
        // 	recv_idle_time: u64,
        // 	rpc_credits_per_hash: u32,
        // 	rpc_port: u16,
        // 	send_count: u64,
        // 	send_idle_time: u64,
        // 	// Exists in the original definition, but isn't
        // 	// used or (de)serialized for RPC purposes.
        // 	// ssl: bool,
        // 	state: ConnectionState,
        // 	support_flags: u32,
        // }
    }
}

impl From<SetBan> for crate::misc::SetBan {
    fn from(x: SetBan) -> Self {
        todo!();

        // Self {
        // 	#[cfg_attr(feature = "serde", serde(default = "default_string"))]
        // 	host: String,
        // 	#[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        // 	ip: u32,
        // 	ban: bool,
        // 	seconds: u32,
        // }
    }
}

impl From<GetBan> for crate::misc::GetBan {
    fn from(x: GetBan) -> Self {
        todo!();

        // Self {
        // 	host: String,
        // 	ip: u32,
        // 	seconds: u32,
        // }
    }
}

impl From<HistogramEntry> for crate::misc::HistogramEntry {
    fn from(x: HistogramEntry) -> Self {
        todo!();

        // Self {
        // 	amount: u64,
        // 	total_instances: u64,
        // 	unlocked_instances: u64,
        // 	recent_instances: u64,
        // }
    }
}

impl From<HardforkEntry> for crate::misc::HardforkEntry {
    fn from(x: HardforkEntry) -> Self {
        todo!();

        // Self {
        // 	height: u64,
        // 	hf_version: u8,
        // }
    }
}

impl From<ChainInfo> for crate::misc::ChainInfo {
    fn from(x: ChainInfo) -> Self {
        todo!();

        // Self {
        // 	block_hash: [u8; 32],
        // 	block_hashes: Vec<[u8; 32]>,
        // 	difficulty_top64: u64,
        // 	difficulty_low64: u64,
        // 	height: u64,
        // 	length: u64,
        // 	main_chain_parent_block: [u8; 32],
        // }
    }
}

impl From<SyncInfoPeer> for crate::misc::SyncInfoPeer {
    fn from(x: SyncInfoPeer) -> Self {
        todo!();

        // Self {
        // 	info: ConnectionInfo,
        // }
    }
}

impl From<Span> for crate::misc::Span {
    fn from(x: Span) -> Self {
        todo!();

        // Self {
        // 	connection_id: String,
        // 	nblocks: u64,
        // 	rate: u32,
        // 	remote_address: String,
        // 	size: u64,
        // 	speed: u32,
        // 	start_block_height: u64,
        // }
    }
}

impl From<TxBacklogEntry> for crate::misc::TxBacklogEntry {
    fn from(x: TxBacklogEntry) -> Self {
        todo!();

        // Self {
        // 	weight: u64,
        // 	fee: u64,
        // 	time_in_pool: u64,
        // }
    }
}

impl From<OutputDistributionData> for crate::misc::OutputDistributionData {
    fn from(x: OutputDistributionData) -> Self {
        todo!();

        // Self {
        // 	distribution: Vec<u64>,
        // 	start_height: u64,
        // 	base: u64,
        // }
    }
}

impl From<GetMinerDataTxBacklogEntry> for crate::misc::GetMinerDataTxBacklogEntry {
    fn from(x: GetMinerDataTxBacklogEntry) -> Self {
        todo!();

        // Self {
        // 	id: String,
        // 	weight: u64,
        // 	fee: u64,
        // }
    }
}

impl From<AuxPow> for crate::misc::AuxPow {
    fn from(x: AuxPow) -> Self {
        todo!();

        // Self {
        // 	id: [u8; 32],
        // 	hash: [u8; 32],
        // }
    }
}

impl From<TxOutputIndices> for crate::misc::TxOutputIndices {
    fn from(x: TxOutputIndices) -> Self {
        todo!();

        // Self {
        // 	indices: Vec<u64>,
        // }
    }
}

impl From<BlockOutputIndices> for crate::misc::BlockOutputIndices {
    fn from(x: BlockOutputIndices) -> Self {
        todo!();

        // Self {
        // 	indices: Vec<TxOutputIndices>,
        // }
    }
}

impl From<GetOutputsOut> for crate::misc::GetOutputsOut {
    fn from(x: GetOutputsOut) -> Self {
        todo!();

        // Self {
        // 	amount: u64,
        // 	index: u64,
        // }
    }
}

impl From<OutKeyBin> for crate::misc::OutKeyBin {
    fn from(x: OutKeyBin) -> Self {
        todo!();

        // Self {
        // 	key: [u8; 32],
        // 	mask: [u8; 32],
        // 	unlocked: bool,
        // 	height: u64,
        // 	txid: [u8; 32],
        // }
    }
}

impl From<Peer> for crate::misc::Peer {
    fn from(x: Peer) -> Self {
        todo!();

        // Self {
        // 	id: u64,
        // 	host: String,
        // 	ip: u32,
        // 	port: u16,
        // 	#[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        // 	rpc_port: u16 = default_zero::<u16>(),
        // 	#[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        // 	rpc_credits_per_hash: u32 = default_zero::<u32>(),
        // 	last_seen: u64,
        // 	#[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        // 	pruning_seed: u32 = default_zero::<u32>(),
        // }
    }
}

impl From<PublicNode> for crate::misc::PublicNode {
    fn from(x: PublicNode) -> Self {
        todo!();

        // Self {
        // 	host: String,
        // 	last_seen: u64,
        // 	rpc_port: u16,
        // 	rpc_credits_per_hash: u32,
        // }
    }
}

impl From<TxInfo> for crate::misc::TxInfo {
    fn from(x: TxInfo) -> Self {
        todo!();

        // Self {
        // 	blob_size: u64,
        // 	do_not_relay: bool,
        // 	double_spend_seen: bool,
        // 	fee: u64,
        // 	id_hash: String,
        // 	kept_by_block: bool,
        // 	last_failed_height: u64,
        // 	last_failed_id_hash: String,
        // 	last_relayed_time: u64,
        // 	max_used_block_height: u64,
        // 	max_used_block_id_hash: String,
        // 	receive_time: u64,
        // 	relayed: bool,
        // 	tx_blob: String,
        // 	tx_json: String, // TODO: this should be another struct
        // 	#[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        // 	weight: u64 = default_zero::<u64>(),
        // }
    }
}

impl From<SpentKeyImageInfo> for crate::misc::SpentKeyImageInfo {
    fn from(x: SpentKeyImageInfo) -> Self {
        todo!();

        // Self {
        // 	id_hash: String,
        // 	txs_hashes: Vec<String>,
        // }
    }
}

impl From<TxpoolHisto> for crate::misc::TxpoolHisto {
    fn from(x: TxpoolHisto) -> Self {
        todo!();

        // Self {
        // 	txs: u32,
        // 	bytes: u64,
        // }
    }
}

impl From<TxpoolStats> for crate::misc::TxpoolStats {
    fn from(x: TxpoolStats) -> Self {
        todo!();

        // Self {
        // 	bytes_max: u32,
        // 	bytes_med: u32,
        // 	bytes_min: u32,
        // 	bytes_total: u64,
        // 	fee_total: u64,
        // 	histo_98pc: u64,
        // 	histo: Vec<TxpoolHisto>,
        // 	num_10m: u32,
        // 	num_double_spends: u32,
        // 	num_failing: u32,
        // 	num_not_relayed: u32,
        // 	oldest: u64,
        // 	txs_total: u32,
        // }
    }
}

impl From<OutKey> for crate::misc::OutKey {
    fn from(x: OutKey) -> Self {
        todo!();

        // Self {
        // 	key: String,
        // 	mask: String,
        // 	unlocked: bool,
        // 	height: u64,
        // 	txid: String,
        // }
    }
}
