/// Input counts greater than this require PoWER.
pub const POWER_INPUT_THRESHOLD: usize = 8;

/// Number of recent block hashes viable for RPC.
pub const POWER_HEIGHT_WINDOW: usize = 2;

/// Fixed difficulty for the difficulty formula.
///
/// Target time = ~1s of single-threaded computation.
/// The difficulty value and computation time have a quadratic relationship.
/// Reference values; value of machines are measured in seconds:
///
/// | Difficulty | Raspberry Pi 5 | Ryzen 5950x | Mac mini M4 |
/// |------------|----------------|-------------|-------------|
/// | 0          | 0.024          | 0.006       | 0.005       |
/// | 25         | 0.307          | 0.076       | 0.067       |
/// | 50         | 0.832          | 0.207       | 0.187       |
/// | 75         | 1.654          | 0.395       | 0.373       |
/// | 100        | 2.811          | 0.657       | 0.611       |
/// | 125        | 4.135          | 0.995       | 0.918       |
/// | 150        | 5.740          | 1.397       | 1.288       |
/// | 175        | 7.740          | 1.868       | 1.682       |
/// | 200        | 9.935          | 2.365       | 2.140       |
/// | 225        | 12.279         | 2.892       | 2.645       |
/// | 250        | 14.855         | 3.573       | 3.226       |
/// | 275        | 17.736         | 4.378       | 3.768       |
/// | 300        | 20.650         | 5.116       | 4.422       |
pub const POWER_DIFFICULTY: u32 = 100;

/// Max difficulty value.
///
/// Technically, nodes can be modified to send lower/higher difficulties in P2P.
/// A vanilla node will adjust accordingly; it can and will and solve a higher difficulty challenge.
/// This is the max valid difficulty requested from a peer before the connection is dropped.
pub const POWER_MAX_DIFFICULTY: u32 = POWER_DIFFICULTY * 2;

/// Personalization string used in PoWER hashes.
pub const POWER_PERSONALIZATION_STRING: &str = "Monero PoWER";
