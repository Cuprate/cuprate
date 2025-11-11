use crate::config::Config;
use cuprate_database::ConcreteEnv;
use cuprate_linear_tapes::{Advice, LinearTapes, Tape};
use std::iter::{once, Once};

/// The name of the ringCT outputs tape.
pub const RCT_OUTPUTS: &str = "rct_outputs";
/// The pruned blobs tape name.
pub const PRUNED_BLOBS: &str = "pruned_blobs";
/// The names of the prunable tapes, in the order of stripe.
pub const PRUNABLE_BLOBS: [&str; 8] = [
    "prunable1",
    "prunable2",
    "prunable3",
    "prunable4",
    "prunable5",
    "prunable6",
    "prunable7",
    "prunable8",
];
/// The name of the v1 prunable blobs table.
pub const V1_PRUNABLE_BLOBS: &str = "v1_prunable_blobs";
/// The name of the tx infos tape.
pub const TX_INFOS: &str = "tx_infos";
/// The name of the block infos tape.
pub const BLOCK_INFOS: &str = "block_infos";

pub struct Database {
    pub(crate) dynamic_tables: ConcreteEnv,
    pub(crate) linear_tapes: LinearTapes,
}
