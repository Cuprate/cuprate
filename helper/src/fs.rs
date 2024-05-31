//! Cuprate directories and filenames.
//!
//! # Environment variables on Linux
//! Note that this module's functions uses [`dirs`],
//! which adheres to the XDG standard on Linux.
//!
//! This means that the values returned by these functions
//! may change at runtime depending on environment variables,
//! for example:
//!
//! By default the config directory is `~/.config`, however
//! if `$XDG_CONFIG_HOME` is set to something, that will be
//! used instead.
//!
//! ```rust
//! # use cuprate_helper::fs::*;
//! # if cfg!(target_os = "linux") {
//! std::env::set_var("XDG_CONFIG_HOME", "/custom/path");
//! assert_eq!(
//!     cuprate_config_dir().to_string_lossy(),
//!     "/custom/path/cuprate"
//! );
//! # }
//! ```
//!
//! Reference:
//! - <https://github.com/Cuprate/cuprate/issues/46>
//! - <https://docs.rs/dirs>

//---------------------------------------------------------------------------------------------------- Use
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

//---------------------------------------------------------------------------------------------------- Const
/// Cuprate's main directory.
///
/// This is the head PATH node used for any top-level Cuprate directories.
///
/// | OS      | PATH                                                |
/// |---------|-----------------------------------------------------|
/// | Windows | `C:\Users\Alice\AppData\Roaming\Cuprate\`           |
/// | macOS   | `/Users/Alice/Library/Application Support/Cuprate/` |
/// | Linux   | `/home/alice/.config/cuprate/`                      |
///
/// This is shared between all Cuprate programs.
///
/// # Value
/// This is `Cuprate` on `Windows|macOS` and `cuprate` on everything else.
///
/// # Monero Equivalent
/// `.bitmonero`
pub const CUPRATE_DIR: &str = {
    if cfg!(target_os = "windows") || cfg!(target_os = "macos") {
        // The standard for main directories is capitalized.
        "Cuprate"
    } else {
        // Standard on Linux + BSDs is lowercase.
        "cuprate"
    }
};

//---------------------------------------------------------------------------------------------------- Directories
/// Create a (private) `OnceLock` and accessor function for common PATHs used by Cuprate.
///
/// This currently creates these directories:
/// - [`cuprate_cache_dir()`]
/// - [`cuprate_config_dir()`]
/// - [`cuprate_data_dir()`]
/// - [`cuprate_blockchain_dir()`]
///
/// FIXME: Use `LazyLock` when stabilized.
/// <https://github.com/rust-lang/rust/issues/109736>.
/// <https://doc.rust-lang.org/std/sync/struct.LazyLock.html>.
macro_rules! impl_path_oncelock_and_fn {
    ($(
        $(#[$attr:meta])* // Documentation and any `derive`'s.
        $fn:ident,        // Name of the corresponding access function.
        $dirs_fn:ident,   // Name of the `dirs` function to use, the PATH prefix.
        $sub_dirs:literal // Any sub-directories to add onto the PATH.
    ),* $(,)?) => {$(
        // Create the `OnceLock` if needed, append
        // the Cuprate directory string and return.
        $(#[$attr])*
        pub fn $fn() -> &'static Path {
            /// Local `OnceLock` containing the Path.
            static ONCE_LOCK: OnceLock<PathBuf> = OnceLock::new();

            ONCE_LOCK.get_or_init(|| {
                // There's nothing we can do but panic if
                // we cannot acquire critical system directories.
                //
                // Although, this realistically won't panic on
                // normal systems for all OS's supported by `dirs`.
                let mut path = dirs::$dirs_fn().unwrap();

                // FIXME:
                // Consider a user who does `HOME=/ ./cuprated`
                //
                // Should we say "that's stupid" and panic here?
                // Or should it be respected?
                // We really don't want a `rm -rf /` type of situation...
                assert!(
                    path.parent().is_some(),
                    "SAFETY: returned OS PATH was either root or empty, aborting"
                );

                // Returned OS PATH should be absolute, not relative.
                assert!(path.is_absolute(), "SAFETY: returned OS PATH was not absolute");

                // Unconditionally prefix with the top-level Cuprate directory.
                path.push(CUPRATE_DIR);

                // Add any sub directories if specified in the macro.
                if !$sub_dirs.is_empty() {
                    path.push($sub_dirs);
                }

                path
            })
        }
    )*};
}

// Note that the `OnceLock`'s are prefixed with `__` to indicate:
// 1. They're not really to be used directly
// 2. To avoid name conflicts
impl_path_oncelock_and_fn! {
    /// Cuprate's cache directory.
    ///
    /// This is the PATH used for any Cuprate cache files.
    ///
    /// | OS      | PATH                                    |
    /// |---------|-----------------------------------------|
    /// | Windows | `C:\Users\Alice\AppData\Local\Cuprate\` |
    /// | macOS   | `/Users/Alice/Library/Caches/Cuprate/`  |
    /// | Linux   | `/home/alice/.cache/cuprate/`           |
    cuprate_cache_dir,
    cache_dir,
    "",

    /// Cuprate's config directory.
    ///
    /// This is the PATH used for any Cuprate configuration files.
    ///
    /// | OS      | PATH                                                |
    /// |---------|-----------------------------------------------------|
    /// | Windows | `C:\Users\Alice\AppData\Roaming\Cuprate\`           |
    /// | macOS   | `/Users/Alice/Library/Application Support/Cuprate/` |
    /// | Linux   | `/home/alice/.config/cuprate/`                      |
    cuprate_config_dir,
    config_dir,
    "",

    /// Cuprate's data directory.
    ///
    /// This is the PATH used for any Cuprate data files.
    ///
    /// | OS      | PATH                                                |
    /// |---------|-----------------------------------------------------|
    /// | Windows | `C:\Users\Alice\AppData\Roaming\Cuprate\`           |
    /// | macOS   | `/Users/Alice/Library/Application Support/Cuprate/` |
    /// | Linux   | `/home/alice/.local/share/cuprate/`                 |
    cuprate_data_dir,
    data_dir,
    "",

    /// Cuprate's blockchain directory.
    ///
    /// This is the PATH used for any Cuprate blockchain files.
    ///
    /// | OS      | PATH                                                           |
    /// |---------|----------------------------------------------------------------|
    /// | Windows | `C:\Users\Alice\AppData\Roaming\Cuprate\blockchain\`           |
    /// | macOS   | `/Users/Alice/Library/Application Support/Cuprate/blockchain/` |
    /// | Linux   | `/home/alice/.local/share/cuprate/blockchain/`                 |
    cuprate_blockchain_dir,
    data_dir,
    "blockchain",
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    // Sanity check every PATH defined in this file.
    //
    // Each new PATH should be added to this test:
    // - It must be `is_absolute()`
    // - It must `ends_with()` the expected end PATH for the OS
    #[test]
    fn path_sanity_check() {
        assert!(cuprate_cache_dir().is_absolute());
        assert!(cuprate_config_dir().is_absolute());
        assert!(cuprate_data_dir().is_absolute());
        assert!(cuprate_blockchain_dir().is_absolute());

        if cfg!(target_os = "windows") {
            let dir = cuprate_cache_dir();
            println!("cuprate_cache_dir: {dir:?}");
            assert!(dir.ends_with(r"AppData\Local\Cuprate"));

            let dir = cuprate_config_dir();
            println!("cuprate_config_dir: {dir:?}");
            assert!(dir.ends_with(r"AppData\Roaming\Cuprate"));

            let dir = cuprate_data_dir();
            println!("cuprate_data_dir: {dir:?}");
            assert!(dir.ends_with(r"AppData\Roaming\Cuprate"));

            let dir = cuprate_blockchain_dir();
            println!("cuprate_blockchain_dir: {dir:?}");
            assert!(dir.ends_with(r"AppData\Roaming\Cuprate\blockchain"));
        } else if cfg!(target_os = "macos") {
            let dir = cuprate_cache_dir();
            println!("cuprate_cache_dir: {dir:?}");
            assert!(dir.ends_with("Library/Caches/Cuprate"));

            let dir = cuprate_config_dir();
            println!("cuprate_config_dir: {dir:?}");
            assert!(dir.ends_with("Library/Application Support/Cuprate"));

            let dir = cuprate_data_dir();
            println!("cuprate_data_dir: {dir:?}");
            assert!(dir.ends_with("Library/Application Support/Cuprate"));

            let dir = cuprate_blockchain_dir();
            println!("cuprate_blockchain_dir: {dir:?}");
            assert!(dir.ends_with("Library/Application Support/Cuprate/blockchain"));
        } else {
            // Assumes Linux.
            let dir = cuprate_cache_dir();
            println!("cuprate_cache_dir: {dir:?}");
            assert!(dir.ends_with(".cache/cuprate"));

            let dir = cuprate_config_dir();
            println!("cuprate_config_dir: {dir:?}");
            assert!(dir.ends_with(".config/cuprate"));

            let dir = cuprate_data_dir();
            println!("cuprate_data_dir: {dir:?}");
            assert!(dir.ends_with(".local/share/cuprate"));

            let dir = cuprate_blockchain_dir();
            println!("cuprate_blockchain_dir: {dir:?}");
            assert!(dir.ends_with(".local/share/cuprate/blockchain"));
        }
    }
}
