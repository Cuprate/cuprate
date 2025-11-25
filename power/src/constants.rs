/// Input counts greater than this require PoWER.
pub const POWER_INPUT_THRESHOLD: usize = 8;

/// Number of recent block hashes viable for RPC.
pub const POWER_HEIGHT_WINDOW: usize = 2;

/// Fixed difficulty for valid PoW.
pub const POWER_DIFFICULTY: u32 = 20;

/// Personalization string used in PoWER hashes.
pub const POWER_CHALLENGE_PERSONALIZATION_STRING: &str = "Monero PoWER";
