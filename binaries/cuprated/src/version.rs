//! Misc version information about `cuprated`.

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use cuprate_constants::build::{BUILD, COMMIT};
use cuprate_rpc_types::{CORE_RPC_VERSION, CORE_RPC_VERSION_MAJOR, CORE_RPC_VERSION_MINOR};
use cuprate_types::HardFork;

use crate::constants::{MAJOR_VERSION, MINOR_VERSION, PATCH_VERSION, VERSION};

// NOTE: keep `books/user/src/cli.md` updated with this struct.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CupratedVersionInfo {
    /// `cuprated`'s major version.
    major_version: u8,
    /// `cuprated`'s minor version.
    minor_version: u8,
    /// `cuprated`'s patch version.
    patch_version: u8,
    /// RPC major version (from `monerod`).
    rpc_major_version: u32,
    /// RPC minor version (from `monerod`).
    rpc_minor_version: u32,
    /// RPC version (from `monerod`).
    rpc_version: u32,
    /// The latest hardfork supported.
    hardfork: u8,
    /// The blockchain database version specific to `cuprated`.
    blockchain_db_version: u64,
    /// `cuprated`'s semantic version.
    semantic_version: &'static str,
    /// Build type, either `debug` or `release`.
    build: &'static str,
    /// Git commit hash of the build.
    commit: &'static str,
}

impl CupratedVersionInfo {
    /// Generate version info.
    pub fn new() -> Self {
        Self {
            major_version: MAJOR_VERSION.parse().unwrap(),
            minor_version: MINOR_VERSION.parse().unwrap(),
            patch_version: PATCH_VERSION.parse().unwrap(),
            rpc_major_version: CORE_RPC_VERSION_MAJOR,
            rpc_minor_version: CORE_RPC_VERSION_MINOR,
            rpc_version: CORE_RPC_VERSION,
            blockchain_db_version: cuprate_blockchain::DATABASE_VERSION,
            hardfork: HardFork::LATEST.as_u8(),
            semantic_version: VERSION,
            build: BUILD,
            commit: COMMIT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CupratedVersionInfo;

    /// Tests that [`CupratedVersionInfo`] can be generated.
    #[test]
    fn new() {
        CupratedVersionInfo::new();
    }
}
