//! Cuprate directories and filenames.

//---------------------------------------------------------------------------------------------------- Use
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

//---------------------------------------------------------------------------------------------------- Const
/// Cuprate's main directory.
///
/// This is the PATH used for any top-level Cuprate directories.
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

/// Attempt to create all Cuprate directories.
///
/// This currently creates these directories:
/// - [`cuprate_cache_dir()`]
/// - [`cuprate_config_dir()`]
/// - [`cuprate_data_dir()`]
///
/// # Errors
/// This will return early if any of the above functions error.
pub fn cuprate_create_dir_all() -> std::io::Result<()> {
    for path in [
        cuprate_cache_dir(),
        cuprate_config_dir(),
        cuprate_data_dir(),
    ] {
        std::fs::create_dir_all(path)?;
    }

    Ok(())
}

//---------------------------------------------------------------------------------------------------- Directories
/// Create a (private) `OnceLock` and accessor function for common PATHs used by Cuprate.
///
/// This creates all the functions used in [`cuprate_create_dir_all`].
macro_rules! impl_dir_oncelock_and_fn {
    ($(
        $(#[$attr:meta])* // Documentation and any `derive`'s.
        $fn:ident,        // Name of the corresponding access function.
        $dirs_fn:ident,   // Name of the `dirs` function to use, the PATH prefix.
        $once_lock:ident, // Name of the `OnceLock`.
        $expect:literal   // Panic message if directory get fails.
    ),* $(,)?) => {$(
        /// Local `OnceLock` containing the Path.
        static $once_lock: OnceLock<PathBuf> = OnceLock::new();

        // Create the `OnceLock` if needed, append
        // the Cuprate directory string and return.
        $(#[$attr])*
        pub fn $fn() -> &'static Path {
            $once_lock.get_or_init(|| {
                // This should never panic.
                let mut path = dirs::$dirs_fn().expect($expect);

                // TODO:
                // Consider a user who does `HOME=/ ./cuprated`
                //
                // Should we say "that's stupid" and panic here?
                // Or should it be respected?
                // We really don't want a `rm -rf /` type of situation...
                assert!(
                    !path.parent().is_some(),
                    "SAFETY: returned OS directory was either root or empty, aborting"
                );

                path.push(CUPRATE_DIR);
                path
            })
        }
    )*};
}

impl_dir_oncelock_and_fn! {
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
    CUPRATE_CACHE_DIR,
    "Cache directory was not found",

    /// Cuprate's cache directory.
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
    CUPRATE_CONFIG_DIR,
    "Configuration directory was not found",

    /// Cuprate's cache directory.
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
    CUPRATE_DATA_DIR,
    "Data directory was not found",
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
