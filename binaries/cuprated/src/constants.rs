//! General constants used throughout `cuprated`.

use const_format::formatcp;

/// `cuprated`'s semantic version (`MAJOR.MINOR.PATCH`) as string.
pub const VERSION: &str = clap::crate_version!();

/// [`VERSION`] + the build type.
///
/// If a debug build, the suffix is `-debug`, else it is `-release`.
pub const VERSION_BUILD: &str = if cfg!(debug_assertions) {
    formatcp!("{VERSION}-debug")
} else {
    formatcp!("{VERSION}-release")
};

pub const PANIC_CRITICAL_SERVICE_ERROR: &str =
    "A service critical to Cuprate's function returned an unexpected error.";

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn version() {
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
