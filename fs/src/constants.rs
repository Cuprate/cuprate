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

/// The default name of Cuprate's config file.
pub const DEFAULT_CONFIG_FILE_NAME: &str = "Cuprated.toml";
