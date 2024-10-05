use std::time::Duration;

/// The timeout we set on handshakes.
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(20);

/// The timeout we set on receiving ping requests
pub(crate) const PING_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// The amount of concurrency (maximum number of simultaneous tasks) we allow for handling ping requests
pub(crate) const PING_REQUEST_CONCURRENCY: usize = 2;

/// The maximum amount of connections to make to seed nodes for when we need peers.
pub(crate) const MAX_SEED_CONNECTIONS: usize = 3;

/// The timeout for when we fail to find a peer to connect to.
pub(crate) const OUTBOUND_CONNECTION_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(5);

/// The durations of a short ban.
pub const SHORT_BAN: Duration = Duration::from_secs(60 * 10);

/// The durations of a medium ban.
pub const MEDIUM_BAN: Duration = Duration::from_secs(60 * 60 * 24);

/// The durations of a long ban.
pub const LONG_BAN: Duration = Duration::from_secs(60 * 60 * 24 * 7);

/// The default amount of time between inbound diffusion flushes.
pub(crate) const DIFFUSION_FLUSH_AVERAGE_SECONDS_INBOUND: Duration = Duration::from_secs(5);

/// The default amount of time between outbound diffusion flushes.
pub(crate) const DIFFUSION_FLUSH_AVERAGE_SECONDS_OUTBOUND: Duration = Duration::from_millis(2500);

/// This size limit on [`NewTransactions`](monero_wire::protocol::NewTransactions) messages that we create.
pub(crate) const SOFT_TX_MESSAGE_SIZE_SIZE_LIMIT: usize = 10 * 1024 * 1024;

/// The amount of transactions in the broadcast queue. When this value is hit, old transactions will be dropped from
/// the queue.
///
/// Because of internal implementation details this value is _always_ hit, i.e. a transaction will not be dropped until
/// 50 more transactions after it are added to the queue.
pub(crate) const MAX_TXS_IN_BROADCAST_CHANNEL: usize = 50;

/// The time to sleep after an inbound connection comes in.
///
/// This is a safety measure to prevent Cuprate from getting spammed with a load of inbound connections.
/// TODO: it might be a good idea to make this configurable.
pub(crate) const INBOUND_CONNECTION_COOL_DOWN: Duration = Duration::from_millis(500);

/// The initial amount of chain requests to send to find the best chain to sync from.
pub(crate) const INITIAL_CHAIN_REQUESTS_TO_SEND: usize = 3;

/// The enforced maximum amount of blocks to request in a batch.
///
/// Requesting more than this will cause the peer to disconnect and potentially lead to bans.
pub const MAX_BLOCK_BATCH_LEN: usize = 100;

/// The timeout that the block downloader will use for requests.
pub(crate) const BLOCK_DOWNLOADER_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The maximum size of a transaction, a sanity limit that all transactions across all hard-forks must
/// be less than.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions.html#transaction-size>
pub(crate) const MAX_TRANSACTION_BLOB_SIZE: usize = 1_000_000;

/// The maximum amount of block IDs allowed in a chain entry response.
///
/// ref: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_config.h#L97>
// TODO: link to the protocol book when this section is added.
pub(crate) const MAX_BLOCKS_IDS_IN_CHAIN_ENTRY: usize = 25_000;

/// The amount of failures downloading a specific batch before we stop attempting to download it.
pub(crate) const MAX_DOWNLOAD_FAILURES: usize = 5;

/// The amount of empty chain entries to receive before we assume we have found the top of the chain.
pub(crate) const EMPTY_CHAIN_ENTRIES_BEFORE_TOP_ASSUMED: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    /// Outbound diffusion flushes should be shorter than
    /// inbound ones as we control these connections.
    #[test]
    fn outbound_diffusion_flush_shorter_than_inbound() {
        assert!(DIFFUSION_FLUSH_AVERAGE_SECONDS_OUTBOUND < DIFFUSION_FLUSH_AVERAGE_SECONDS_INBOUND);
    }

    /// Checks that the ban time increases from short to long.
    #[test]
    fn ban_times_sanity_check() {
        assert!(SHORT_BAN < MEDIUM_BAN && MEDIUM_BAN < LONG_BAN);
    }
}
