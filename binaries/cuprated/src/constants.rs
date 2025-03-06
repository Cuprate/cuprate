//! General constants used throughout `cuprated`.

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

pub const EXAMPLE_CONFIG: &str = include_str!("../config/Cuprated.toml");

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Config;

    #[test]
    fn version() {
        let semantic_version = format!("{MAJOR_VERSION}.{MINOR_VERSION}.{PATCH_VERSION}");
        assert_eq!(VERSION, VERSION);
        assert_eq!(VERSION, "0.0.1");
    }

    #[test]
    fn version_build() {
        if cfg!(debug_assertions) {
            assert_eq!(VERSION_BUILD, "0.0.1-debug");
        } else {
            assert_eq!(VERSION_BUILD, "0.0.1-release");
        }
    }
}
