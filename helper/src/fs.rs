//! Cuprate directories and filenames.
//!
//! # Environment variables on Linux
//! Note that this module's functions uses [`dirs`],
//! which adheres to the XDG standard on Linux.
//!
//! This means that the values returned by these statics
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
//!     CUPRATE_CONFIG_DIR.to_string_lossy(),
//!     "/custom/path/cuprate"
//! );
//! # }
//! ```
//!
//! Reference:
//! - <https://github.com/Cuprate/cuprate/issues/46>
//! - <https://docs.rs/dirs>

//---------------------------------------------------------------------------------------------------- Use
use std::{path::PathBuf, sync::LazyLock};

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
/// Create a `LazyLock` for common PATHs used by Cuprate.
///
/// This currently creates these directories:
/// - [`CUPRATE_CACHE_DIR`]
/// - [`CUPRATE_CONFIG_DIR`]
/// - [`CUPRATE_DATA_DIR`]
/// - [`CUPRATE_BLOCKCHAIN_DIR`]
macro_rules! impl_path_lazylock {
    ($(
        $(#[$attr:meta])* // Documentation and any `derive`'s.
        $name:ident,        // Name of the corresponding `LazyLock`.
        $dirs_fn:ident,   // Name of the `dirs` function to use, the PATH prefix.
        $sub_dirs:literal // Any sub-directories to add onto the PATH.
    ),* $(,)?) => {$(
        // Create the `LazyLock` if needed, append
        // the Cuprate directory string and return.
        $(#[$attr])*
        pub static $name: LazyLock<PathBuf> = LazyLock::new(|| {
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
        });
    )*};
}

impl_path_lazylock! {
    /// Cuprate's cache directory.
    ///
    /// This is the PATH used for any Cuprate cache files.
    ///
    /// | OS      | PATH                                    |
    /// |---------|-----------------------------------------|
    /// | Windows | `C:\Users\Alice\AppData\Local\Cuprate\` |
    /// | macOS   | `/Users/Alice/Library/Caches/Cuprate/`  |
    /// | Linux   | `/home/alice/.cache/cuprate/`           |
    CUPRATE_CACHE_DIR,
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
    CUPRATE_CONFIG_DIR,
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
    CUPRATE_DATA_DIR,
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
    CUPRATE_BLOCKCHAIN_DIR,
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
        assert!(CUPRATE_CACHE_DIR.is_absolute());
        assert!(CUPRATE_CONFIG_DIR.is_absolute());
        assert!(CUPRATE_DATA_DIR.is_absolute());
        assert!(CUPRATE_BLOCKCHAIN_DIR.is_absolute());

        if cfg!(target_os = "windows") {
            let dir = &*CUPRATE_CACHE_DIR;
            println!("cuprate_cache_dir: {dir:?}");
            assert!(dir.ends_with(r"AppData\Local\Cuprate"));

            let dir = &*CUPRATE_CONFIG_DIR;
            println!("cuprate_config_dir: {dir:?}");
            assert!(dir.ends_with(r"AppData\Roaming\Cuprate"));

            let dir = &*CUPRATE_DATA_DIR;
            println!("cuprate_data_dir: {dir:?}");
            assert!(dir.ends_with(r"AppData\Roaming\Cuprate"));

            let dir = &*CUPRATE_BLOCKCHAIN_DIR;
            println!("cuprate_blockchain_dir: {dir:?}");
            assert!(dir.ends_with(r"AppData\Roaming\Cuprate\blockchain"));
        } else if cfg!(target_os = "macos") {
            let dir = &*CUPRATE_CACHE_DIR;
            println!("cuprate_cache_dir: {dir:?}");
            assert!(dir.ends_with("Library/Caches/Cuprate"));

            let dir = &*CUPRATE_CONFIG_DIR;
            println!("cuprate_config_dir: {dir:?}");
            assert!(dir.ends_with("Library/Application Support/Cuprate"));

            let dir = &*CUPRATE_DATA_DIR;
            println!("cuprate_data_dir: {dir:?}");
            assert!(dir.ends_with("Library/Application Support/Cuprate"));

            let dir = &*CUPRATE_BLOCKCHAIN_DIR;
            println!("cuprate_blockchain_dir: {dir:?}");
            assert!(dir.ends_with("Library/Application Support/Cuprate/blockchain"));
        } else {
            // Assumes Linux.
            let dir = &*CUPRATE_CACHE_DIR;
            println!("cuprate_cache_dir: {dir:?}");
            assert!(dir.ends_with(".cache/cuprate"));

            let dir = &*CUPRATE_CONFIG_DIR;
            println!("cuprate_config_dir: {dir:?}");
            assert!(dir.ends_with(".config/cuprate"));

            let dir = &*CUPRATE_DATA_DIR;
            println!("cuprate_data_dir: {dir:?}");
            assert!(dir.ends_with(".local/share/cuprate"));

            let dir = &*CUPRATE_BLOCKCHAIN_DIR;
            println!("cuprate_blockchain_dir: {dir:?}");
            assert!(dir.ends_with(".local/share/cuprate/blockchain"));
        }
    }
}
