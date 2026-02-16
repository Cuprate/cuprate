//! General constants used throughout `cuprated`.
use std::time::Duration;

use const_format::formatcp;

/// `cuprated`'s semantic version (`MAJOR.MINOR.PATCH`) as string.
pub const VERSION: &str = clap::crate_version!();

/// Major version number of `cuprated`.
pub const MAJOR_VERSION: &str = env!("CARGO_PKG_VERSION_MAJOR");

/// Major version number of `cuprated`.
pub const MINOR_VERSION: &str = env!("CARGO_PKG_VERSION_MINOR");

/// Patch version number of `cuprated`.
pub const PATCH_VERSION: &str = env!("CARGO_PKG_VERSION_PATCH");

/// [`VERSION`] + the build type.
///
/// If a debug build, the suffix is `-debug`, else it is `-release`.
pub const VERSION_BUILD: &str = formatcp!("{VERSION}-{}", cuprate_constants::build::BUILD);

/// The panic message used when cuprated encounters a critical service error.
pub const PANIC_CRITICAL_SERVICE_ERROR: &str =
    "A service critical to Cuprate's function returned an unexpected error.";

pub const DEFAULT_CONFIG_WARNING: &str = formatcp!(
    "WARNING: no config file found, using default config.\
    \nThe default config may not be optimal for your setup, see the user book here: https://user.cuprate.org/.\
    \nPausing startup for {} seconds. \
    \nUse the `--skip-config-warning` arg to skip this delay if you really want to use the default.",
    DEFAULT_CONFIG_STARTUP_DELAY.as_secs()
);

pub const DEFAULT_CONFIG_STARTUP_DELAY: Duration = Duration::from_secs(15);

// TODO:
pub const DATABASE_CORRUPT_MSG: &str = "Failed to initialize database, database may be corrupted";

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Config;

    #[test]
    fn version() {
        let semantic_version = format!("{MAJOR_VERSION}.{MINOR_VERSION}.{PATCH_VERSION}");
        assert_eq!(VERSION, VERSION);
        assert_eq!(VERSION, "0.0.8");
    }

    #[test]
    fn version_build() {
        if cfg!(debug_assertions) {
            assert_eq!(VERSION_BUILD, "0.0.8-debug");
        } else {
            assert_eq!(VERSION_BUILD, "0.0.8-release");
        }
    }
}
