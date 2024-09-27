//! - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/version.h>
//! - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/version.cpp.in>

use const_format::formatcp;

pub const CUPRATED_VERSION: &str = formatcp!("{}", clap::crate_version!());
pub const CUPRATED_RELEASE_NAME: &str = "Fluorine Fermi";
pub const CUPRATED_VERSION_IS_RELEASE: bool = !cfg!(debug_assertions);
