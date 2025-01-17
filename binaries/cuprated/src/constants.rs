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

/// The panic message used when cuprated encounters a critical service error.
pub const PANIC_CRITICAL_SERVICE_ERROR: &str =
    "A service critical to Cuprate's function returned an unexpected error.";

pub const EXAMPLE_CONFIG: &str = include_str!("../Cuprated.toml");

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Config;

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

    #[test]
    fn generate_config_text_is_valid() {
        let config: Config = toml::from_str(EXAMPLE_CONFIG).unwrap();

        assert_eq!(config, Config::default());
    }
}
