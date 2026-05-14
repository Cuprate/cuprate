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

/// The message used when cuprated encounters a critical service error.
pub const CRITICAL_SERVICE_ERROR: &str =
    "A service critical to Cuprate's function returned an unexpected error.";

pub const DEFAULT_CONFIG_WARNING: &str = formatcp!(
    "WARNING: no config file found, using default config.\
    \nThe default config may not be optimal for your setup, see the user book here: https://user.cuprate.org/.\
    \nPausing startup for {} seconds. \
    \nUse the `--skip-config-warning` arg to skip this delay if you really want to use the default.",
    DEFAULT_CONFIG_STARTUP_DELAY.as_secs()
);

pub const DEFAULT_CONFIG_STARTUP_DELAY: Duration = Duration::from_secs(15);

/// Corrupt database error message.
///
/// The error message shown to end-users in panic
/// messages if we think the database is corrupted.
///
/// This is meant to be user-friendly.
pub const DATABASE_CORRUPT_MSG: &str = r"`cuprated` has encountered a fatal error. The database may be corrupted.

If `cuprated` continues to crash with the current database,
you may have to delete the database file and re-sync from scratch.

See <https://user.cuprate.org/resources/disk.html>
for more information on where database files are.

If this happens frequently, consider using the `Safe` sync mode.";

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Config;

    #[test]
    fn version() {
        let semantic_version = format!("{MAJOR_VERSION}.{MINOR_VERSION}.{PATCH_VERSION}");
        assert_eq!(VERSION, VERSION);
        assert_eq!(VERSION, "0.0.9");
    }

    #[test]
    fn version_build() {
        if cfg!(debug_assertions) {
            assert_eq!(VERSION_BUILD, "0.0.9-debug");
        } else {
            assert_eq!(VERSION_BUILD, "0.0.9-release");
        }
    }
}
