/// Input counts greater than this require PoWER.
pub const POWER_INPUT_THRESHOLD: usize = 8;

/// Number of recent block hashes viable for RPC.
pub const POWER_HEIGHT_WINDOW: usize = 2;

/// Fixed difficulty for valid PoW.
///
/// Target time = ~1s of single-threaded computation.
///
/// The difficulty value and computation time
/// are directly proportional, in other words:
/// - `DIFFICULTY = 200` takes twice as long as
/// - `DIFFICULTY = 100` takes twice as long as
/// - `DIFFICULTY = 50` and so on
///
/// Reference values; value of machines are measured in
/// seconds and rounded to the expected average given
/// enough computation:
///
/// | Difficulty | Raspberry Pi 5 | Ryzen 5950x | Mac mini M4 |
/// |------------|----------------|-------------|-------------|
/// | 25         | 0.6            | 0.15        | 0.09        |
/// | 50         | 1.2            | 0.30        | 0.18        |
/// | 100        | 2.4            | 0.60        | 0.37        |
/// | 200        | 4.8            | 1.20        | 0.75        |
/// | 400        | 9.6            | 2.40        | 1.50        |
pub const POWER_DIFFICULTY: u32 = 200;

/// Max difficulty value.
///
/// Technically, nodes can be modified to send lower/higher difficulties in P2P.
/// A vanilla node will adjust accordingly; it can and will and solve a higher difficulty challenge.
/// This is the max valid difficulty requested from a peer before the connection is dropped.
pub const POWER_MAX_DIFFICULTY: u32 = POWER_DIFFICULTY * 2;

/// Personalization string used in PoWER hashes.
pub const POWER_PERSONALIZATION_STRING: &str = "Monero PoWER";
