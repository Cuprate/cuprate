//! TODO

use crate::macros::monero_definition_link;

/// TODO
///
#[doc = monero_definition_link!()]
pub const CRYPTONOTE_MAX_BLOCK_HEIGHT: usize = 500_000_000;

/// The default log stripes for Monero pruning.
pub const CRYPTONOTE_PRUNING_LOG_STRIPES: u32 = 3;

/// The amount of blocks that peers keep before another stripe starts storing blocks.
pub const CRYPTONOTE_PRUNING_STRIPE_SIZE: usize = 4096;

/// The amount of blocks from the top of the chain that should not be pruned.
pub const CRYPTONOTE_PRUNING_TIP_BLOCKS: usize = 5500;

/// TODO
pub const CRYPTONOTE_DNS_TIMEOUT_MS: u64 = 20000;

/// TODO
pub const CRYPTONOTE_MAX_BLOCK_NUMBER: u64 = 500000000;

/// TODO
pub const CRYPTONOTE_MAX_TX_SIZE: u64 = 1000000;

/// TODO
pub const CRYPTONOTE_MAX_TX_PER_BLOCK: u64 = 0x10000000;

/// TODO
pub const CRYPTONOTE_PUBLIC_ADDRESS_TEXTBLOB_VER: u64 = 0;

/// TODO
pub const CRYPTONOTE_MINED_MONEY_UNLOCK_WINDOW: u64 = 60;

/// TODO
pub const CURRENT_TRANSACTION_VERSION: u64 = 2;

/// TODO
pub const CURRENT_BLOCK_MAJOR_VERSION: u64 = 1;

/// TODO
pub const CURRENT_BLOCK_MINOR_VERSION: u64 = 0;

/// TODO
pub const CRYPTONOTE_BLOCK_FUTURE_TIME_LIMIT: u64 = 60 * 60 * 2;

/// TODO
pub const CRYPTONOTE_DEFAULT_TX_SPENDABLE_AGE: u64 = 10;

/// TODO
pub const BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW: u64 = 60;

// MONEY_SUPPLY - total number coins to be generated
pub const MONEY_SUPPLY: u64 = u64::MAX - 1;

/// TODO
pub const EMISSION_SPEED_FACTOR_PER_MINUTE: u64 = 20;

/// TODO
pub const FINAL_SUBSIDY_PER_MINUTE: u64 = 300000000000; // 3 * pow(10, 11);

/// TODO
pub const CRYPTONOTE_REWARD_BLOCKS_WINDOW: u64 = 100;

/// TODO
pub const CRYPTONOTE_BLOCK_GRANTED_FULL_REWARD_ZONE_V2: u64 = 60000; //size of block (bytes) after which reward for block calculated ;using block size

/// TODO
pub const CRYPTONOTE_BLOCK_GRANTED_FULL_REWARD_ZONE_V1: u64 = 20000; //size of block (bytes) after which reward for block calculated ;using block size - before first fork

/// TODO
pub const CRYPTONOTE_BLOCK_GRANTED_FULL_REWARD_ZONE_V5: u64 = 300000; //size of block (bytes) after which reward for block calculated ;using block size - second change, from v5

/// TODO
pub const CRYPTONOTE_LONG_TERM_BLOCK_WEIGHT_WINDOW_SIZE: u64 = 100000; // size in blocks of the long term block weight median window;

/// TODO
pub const CRYPTONOTE_SHORT_TERM_BLOCK_WEIGHT_SURGE_FACTOR: u64 = 50;

/// TODO
pub const CRYPTONOTE_COINBASE_BLOB_RESERVED_SIZE: u64 = 600;

/// TODO
pub const CRYPTONOTE_DISPLAY_DECIMAL_POINT: u64 = 12;

// COIN - number of smallest units in one coin
pub const COIN: u64 = 1000000000000; // pow(10, 12);

/// TODO
pub const FEE_PER_KB_OLD: u64 = 10000000000; // pow(10, 10);

/// TODO
pub const FEE_PER_KB: u64 = 2000000000; // 2 * pow(10, 9);

/// TODO
pub const FEE_PER_BYTE: u64 = 300000;

/// TODO
pub const DYNAMIC_FEE_PER_KB_BASE_FEE: u64 = 2000000000; // 2 * pow(10,9);

/// TODO
pub const DYNAMIC_FEE_PER_KB_BASE_BLOCK_REWARD: u64 = 10000000000000; // 10 * pow(10,12);

/// TODO
pub const DYNAMIC_FEE_PER_KB_BASE_FEE_V5: u64 = 2000000000
    * CRYPTONOTE_BLOCK_GRANTED_FULL_REWARD_ZONE_V2
    / CRYPTONOTE_BLOCK_GRANTED_FULL_REWARD_ZONE_V5;

/// TODO
pub const DYNAMIC_FEE_REFERENCE_TRANSACTION_WEIGHT: u64 = 3000;

/// TODO
pub const ORPHANED_BLOCKS_MAX_COUNT: u64 = 100;

/// TODO
pub const DIFFICULTY_TARGET_V2: u64 = 120; // seconds;

/// TODO
pub const DIFFICULTY_TARGET_V1: u64 = 60; // seconds - before first fork;

/// TODO
pub const DIFFICULTY_WINDOW: u64 = 720; // blocks;

/// TODO
pub const DIFFICULTY_LAG: u64 = 15; // !!!;

/// TODO
pub const DIFFICULTY_CUT: u64 = 60; // timestamps to cut after sorting;

/// TODO
pub const DIFFICULTY_BLOCKS_COUNT: u64 = DIFFICULTY_WINDOW + DIFFICULTY_LAG;

/// TODO
pub const CRYPTONOTE_LOCKED_TX_ALLOWED_DELTA_SECONDS_V1: u64 =
    DIFFICULTY_TARGET_V1 * CRYPTONOTE_LOCKED_TX_ALLOWED_DELTA_BLOCKS;

/// TODO
pub const CRYPTONOTE_LOCKED_TX_ALLOWED_DELTA_SECONDS_V2: u64 =
    DIFFICULTY_TARGET_V2 * CRYPTONOTE_LOCKED_TX_ALLOWED_DELTA_BLOCKS;

/// TODO
pub const CRYPTONOTE_LOCKED_TX_ALLOWED_DELTA_BLOCKS: u64 = 1;

/// TODO
pub const DIFFICULTY_BLOCKS_ESTIMATE_TIMESPAN: u64 = DIFFICULTY_TARGET_V1; //just alias; used by tests;

/// TODO
pub const BLOCKS_IDS_SYNCHRONIZING_DEFAULT_COUNT: u64 = 10000; //by default, blocks ids count in synchronizing;

/// TODO
pub const BLOCKS_IDS_SYNCHRONIZING_MAX_COUNT: u64 = 25000; //max blocks ids count in synchronizing;

/// TODO
pub const BLOCKS_SYNCHRONIZING_DEFAULT_COUNT_PRE_V4: u64 = 100; //by default, blocks count in blocks downloading;

/// TODO
pub const BLOCKS_SYNCHRONIZING_DEFAULT_COUNT: u64 = 20; //by default, blocks count in blocks downloading;

/// TODO
pub const BLOCKS_SYNCHRONIZING_MAX_COUNT: u64 = 2048; //must be a power of 2, greater than 128, equal to ;SEEDHASH_EPOCH_BLOCKS

/// TODO
pub const CRYPTONOTE_MEMPOOL_TX_LIVETIME: u64 = (86400 * 3); //seconds, three days;

/// TODO
pub const CRYPTONOTE_MEMPOOL_TX_FROM_ALT_BLOCK_LIVETIME: u64 = 604800; //seconds, one week;

/// TODO
pub const CRYPTONOTE_DANDELIONPP_STEMS: u64 = 2; // number of outgoing stem connections per epoch;

/// TODO
pub const CRYPTONOTE_DANDELIONPP_FLUFF_PROBABILITY: u64 = 20; // out of 100;

/// TODO
pub const CRYPTONOTE_DANDELIONPP_MIN_EPOCH: u64 = 10; // minutes;

/// TODO
pub const CRYPTONOTE_DANDELIONPP_EPOCH_RANGE: u64 = 30; // seconds;

/// TODO
pub const CRYPTONOTE_DANDELIONPP_FLUSH_AVERAGE: u64 = 5; // seconds average for poisson distributed fluff flush;

/// TODO
pub const CRYPTONOTE_DANDELIONPP_EMBARGO_AVERAGE: u64 = 39; // seconds (see tx_pool.cpp for more info);

// see src/cryptonote_protocol/levin_notify.cpp
/// TODO
pub const CRYPTONOTE_NOISE_MIN_EPOCH: u64 = 5; // minutes;

/// TODO
pub const CRYPTONOTE_NOISE_EPOCH_RANGE: u64 = 30; // seconds;

/// TODO
pub const CRYPTONOTE_NOISE_MIN_DELAY: u64 = 10; // seconds;

/// TODO
pub const CRYPTONOTE_NOISE_DELAY_RANGE: u64 = 5; // seconds;

/// TODO
pub const CRYPTONOTE_NOISE_BYTES: u64 = 3 * 1024; // 3 KiB;

/// TODO
pub const CRYPTONOTE_NOISE_CHANNELS: u64 = 2; // Max outgoing connections per zone used for noise/covert sending;

// Both below are in seconds. The idea is to delay forwarding from i2p/tor
// to ipv4/6, such that 2+ incoming connections _could_ have sent the tx
/// TODO
pub const CRYPTONOTE_FORWARD_DELAY_BASE: u64 =
    CRYPTONOTE_NOISE_MIN_DELAY + CRYPTONOTE_NOISE_DELAY_RANGE;

/// TODO
pub const CRYPTONOTE_FORWARD_DELAY_AVERAGE: u64 =
    CRYPTONOTE_FORWARD_DELAY_BASE + (CRYPTONOTE_FORWARD_DELAY_BASE / 2);

/// TODO
pub const CRYPTONOTE_MAX_FRAGMENTS: u64 = 20; // ~20 * NOISE_BYTES max payload size for covert/noise send;

/// TODO
pub const COMMAND_RPC_GET_BLOCKS_FAST_MAX_BLOCK_COUNT: u64 = 1000;
/// TODO
pub const COMMAND_RPC_GET_BLOCKS_FAST_MAX_TX_COUNT: u64 = 20000;
/// TODO
pub const MAX_RPC_CONTENT_LENGTH: u64 = 1048576; // 1 MB

/// TODO
pub const P2P_LOCAL_WHITE_PEERLIST_LIMIT: u64 = 1000;
/// TODO
pub const P2P_LOCAL_GRAY_PEERLIST_LIMIT: u64 = 5000;

/// TODO
pub const P2P_DEFAULT_CONNECTIONS_COUNT: u64 = 12;
/// TODO
pub const P2P_DEFAULT_HANDSHAKE_INTERVAL: u64 = 60; //secondes
/// TODO
pub const P2P_DEFAULT_PACKET_MAX_SIZE: u64 = 50000000; //50000000 bytes maximum packet size
/// TODO
pub const P2P_DEFAULT_PEERS_IN_HANDSHAKE: u64 = 250;
/// TODO
pub const P2P_MAX_PEERS_IN_HANDSHAKE: u64 = 250;
/// TODO
pub const P2P_DEFAULT_CONNECTION_TIMEOUT: u64 = 5000; //5 seconds
/// TODO
pub const P2P_DEFAULT_SOCKS_CONNECT_TIMEOUT: u64 = 45; // seconds
/// TODO
pub const P2P_DEFAULT_PING_CONNECTION_TIMEOUT: u64 = 2000; //2 seconds
/// TODO
pub const P2P_DEFAULT_INVOKE_TIMEOUT: u64 = 60 * 2 * 1000; //2 minutes
/// TODO
pub const P2P_DEFAULT_HANDSHAKE_INVOKE_TIMEOUT: u64 = 5000; //5 seconds
/// TODO
pub const P2P_DEFAULT_WHITELIST_CONNECTIONS_PERCENT: u64 = 70;
/// TODO
pub const P2P_DEFAULT_ANCHOR_CONNECTIONS_COUNT: u64 = 2;
/// TODO
pub const P2P_DEFAULT_SYNC_SEARCH_CONNECTIONS_COUNT: u64 = 2;
/// TODO
pub const P2P_DEFAULT_LIMIT_RATE_UP: u64 = 2048; // kB/s
/// TODO
pub const P2P_DEFAULT_LIMIT_RATE_DOWN: u64 = 8192; // kB/s

/// TODO
pub const P2P_FAILED_ADDR_FORGET_SECONDS: u64 = 60 * 60; //1 hour
/// TODO
pub const P2P_IP_BLOCKTIME: u64 = 60 * 60 * 24; //24 hour
/// TODO
pub const P2P_IP_FAILS_BEFORE_BLOCK: u64 = 10;
/// TODO
pub const P2P_IDLE_CONNECTION_KILL_INTERVAL: u64 = 5 * 60; //5 minutes

/// TODO
pub const P2P_SUPPORT_FLAG_FLUFFY_BLOCKS: u64 = 0x01;
/// TODO
pub const P2P_SUPPORT_FLAGS: u64 = P2P_SUPPORT_FLAG_FLUFFY_BLOCKS;

/// TODO
pub const RPC_IP_FAILS_BEFORE_BLOCK: u64 = 3;

/// TODO
pub const THREAD_STACK_SIZE: u64 = 5 * 1024 * 1024;

/// TODO
pub const HF_VERSION_DYNAMIC_FEE: u64 = 4;
/// TODO
pub const HF_VERSION_MIN_MIXIN_4: u64 = 6;
/// TODO
pub const HF_VERSION_MIN_MIXIN_6: u64 = 7;
/// TODO
pub const HF_VERSION_MIN_MIXIN_10: u64 = 8;
/// TODO
pub const HF_VERSION_MIN_MIXIN_15: u64 = 15;
/// TODO
pub const HF_VERSION_ENFORCE_RCT: u64 = 6;
/// TODO
pub const HF_VERSION_PER_BYTE_FEE: u64 = 8;
/// TODO
pub const HF_VERSION_SMALLER_BP: u64 = 10;
/// TODO
pub const HF_VERSION_LONG_TERM_BLOCK_WEIGHT: u64 = 10;
/// TODO
pub const HF_VERSION_MIN_2_OUTPUTS: u64 = 12;
/// TODO
pub const HF_VERSION_MIN_V2_COINBASE_TX: u64 = 12;
/// TODO
pub const HF_VERSION_SAME_MIXIN: u64 = 12;
/// TODO
pub const HF_VERSION_REJECT_SIGS_IN_COINBASE: u64 = 12;
/// TODO
pub const HF_VERSION_ENFORCE_MIN_AGE: u64 = 12;
/// TODO
pub const HF_VERSION_EFFECTIVE_SHORT_TERM_MEDIAN_IN_PENALTY: u64 = 12;
/// TODO
pub const HF_VERSION_EXACT_COINBASE: u64 = 13;
/// TODO
pub const HF_VERSION_CLSAG: u64 = 13;
/// TODO
pub const HF_VERSION_DETERMINISTIC_UNLOCK_TIME: u64 = 13;
/// TODO
pub const HF_VERSION_BULLETPROOF_PLUS: u64 = 15;
/// TODO
pub const HF_VERSION_VIEW_TAGS: u64 = 15;
/// TODO
pub const HF_VERSION_2021_SCALING: u64 = 15;

/// TODO
pub const PER_KB_FEE_QUANTIZATION_DECIMALS: u64 = 8;
/// TODO
pub const CRYPTONOTE_SCALING_2021_FEE_ROUNDING_PLACES: u64 = 2;

/// TODO
pub const HASH_OF_HASHES_STEP: u64 = 512;

/// TODO
pub const DEFAULT_TXPOOL_MAX_WEIGHT: u64 = 648000000; // 3 days at 300000, in bytes

/// TODO
pub const BULLETPROOF_MAX_OUTPUTS: u64 = 16;
/// TODO
pub const BULLETPROOF_PLUS_MAX_OUTPUTS: u64 = 16;

/// TODO
pub const CRYPTONOTE_PRUNING_STRIPE_SIZE: u64 = 4096; // the smaller, the smoother the increase
/// TODO
pub const CRYPTONOTE_PRUNING_LOG_STRIPES: u64 = 3; // the higher, the more space saved
/// TODO
pub const CRYPTONOTE_PRUNING_TIP_BLOCKS: u64 = 5500; // the smaller, the more space saved

/// TODO
pub const RPC_CREDITS_PER_HASH_SCALE: u64 = 1 << 24;

/// TODO
pub const DNS_BLOCKLIST_LIFETIME: u64 = 86400 * 8;

//The limit is enough for the mandatory transaction content with 16 outputs (547 bytes),
//a custom tag (1 byte) and up to 32 bytes of custom data for each recipient.
// (1+32) + (1+1+16*32) + (1+16*32) = 1060
/// TODO
pub const MAX_TX_EXTRA_SIZE: u64 = 1060;
