//! Base paths used by Cuprate.

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

/// Create a `LazyLock` for common PATHs used by Cuprate.
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
}

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
        // Array of (PATH, expected_path_as_string).
        //
        // The different OS's will set the expected path below.
        let mut array = [
            (&*CUPRATE_CACHE_DIR, ""),
            (&*CUPRATE_CONFIG_DIR, ""),
            (&*CUPRATE_DATA_DIR, ""),
        ];

        if cfg!(target_os = "windows") {
            array[0].1 = r"AppData\Local\Cuprate";
            array[1].1 = r"AppData\Roaming\Cuprate";
            array[2].1 = r"AppData\Roaming\Cuprate";
        } else if cfg!(target_os = "macos") {
            array[0].1 = "Library/Caches/Cuprate";
            array[1].1 = "Library/Application Support/Cuprate";
            array[2].1 = "Library/Application Support/Cuprate";
        } else {
            // Assumes Linux.
            array[0].1 = ".cache/cuprate";
            array[1].1 = ".config/cuprate";
            array[2].1 = ".local/share/cuprate";
        }

        for (path, expected) in array {
            assert!(path.is_absolute());
            assert!(path.ends_with(expected));
        }
    }
}
