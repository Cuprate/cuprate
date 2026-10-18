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
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crate::network::Network;

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

/// The default name of Cuprate's config file.
pub const DEFAULT_CONFIG_FILE_NAME: &str = "Cuprated.toml";

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
}

/// Joins the [`Network`] to the [`Path`].
///
/// This will keep the path the same for [`Network::Mainnet`].
pub fn path_with_network(path: &Path, network: Network) -> PathBuf {
    match network {
        Network::Mainnet => path.to_path_buf(),
        Network::Testnet | Network::Stagenet | Network::FakeChain => path.join(network.to_string()),
    }
}

/// Cuprate's blockchain directory.
///
/// This is the PATH used for any Cuprate blockchain files.
///
/// ```rust
/// use cuprate_helper::{network::Network, fs::{CUPRATE_DATA_DIR, blockchain_path}};
///
/// assert_eq!(blockchain_path(&**CUPRATE_DATA_DIR, Network::Mainnet).as_path(), CUPRATE_DATA_DIR.join("blockchain"));
/// assert_eq!(blockchain_path(&**CUPRATE_DATA_DIR, Network::Stagenet).as_path(), CUPRATE_DATA_DIR.join(Network::Stagenet.to_string()).join("blockchain"));
/// assert_eq!(blockchain_path(&**CUPRATE_DATA_DIR, Network::Testnet).as_path(), CUPRATE_DATA_DIR.join(Network::Testnet.to_string()).join("blockchain"));
/// ```
pub fn blockchain_path(data_dir: &Path, network: Network) -> PathBuf {
    path_with_network(data_dir, network).join("blockchain")
}

/// Cuprate's txpool directory.
///
/// This is the PATH used for any Cuprate txpool files.
///
/// ```rust
/// use cuprate_helper::{network::Network, fs::{CUPRATE_DATA_DIR, txpool_path}};
///
/// assert_eq!(txpool_path(&**CUPRATE_DATA_DIR, Network::Mainnet).as_path(), CUPRATE_DATA_DIR.join("txpool"));
/// assert_eq!(txpool_path(&**CUPRATE_DATA_DIR, Network::Stagenet).as_path(), CUPRATE_DATA_DIR.join(Network::Stagenet.to_string()).join("txpool"));
/// assert_eq!(txpool_path(&**CUPRATE_DATA_DIR, Network::Testnet).as_path(), CUPRATE_DATA_DIR.join(Network::Testnet.to_string()).join("txpool"));
/// ```
pub fn txpool_path(data_dir: &Path, network: Network) -> PathBuf {
    path_with_network(data_dir, network).join("txpool")
}

/// Cuprate's logs directory.
///
/// This is the PATH used for all Cuprate log files.
///
/// ```rust
/// use cuprate_helper::{network::Network, fs::{CUPRATE_DATA_DIR, logs_path}};
///
/// assert_eq!(logs_path(&**CUPRATE_DATA_DIR, Network::Mainnet).as_path(), CUPRATE_DATA_DIR.join("logs"));
/// assert_eq!(logs_path(&**CUPRATE_DATA_DIR, Network::Stagenet).as_path(), CUPRATE_DATA_DIR.join(Network::Stagenet.to_string()).join("logs"));
/// assert_eq!(logs_path(&**CUPRATE_DATA_DIR, Network::Testnet).as_path(), CUPRATE_DATA_DIR.join(Network::Testnet.to_string()).join("logs"));
/// ```
pub fn logs_path(data_dir: &Path, network: Network) -> PathBuf {
    path_with_network(data_dir, network).join("logs")
}

/// Cuprate's address-book directory.
///
/// This is the PATH used for any Cuprate address-book files.
///
/// ```rust
/// use cuprate_helper::{network::Network, fs::{CUPRATE_CACHE_DIR, address_book_path}};
///
/// assert_eq!(address_book_path(&**CUPRATE_CACHE_DIR, Network::Mainnet).as_path(), CUPRATE_CACHE_DIR.join("addressbook"));
/// assert_eq!(address_book_path(&**CUPRATE_CACHE_DIR, Network::Stagenet).as_path(), CUPRATE_CACHE_DIR.join(Network::Stagenet.to_string()).join("addressbook"));
/// assert_eq!(address_book_path(&**CUPRATE_CACHE_DIR, Network::Testnet).as_path(), CUPRATE_CACHE_DIR.join(Network::Testnet.to_string()).join("addressbook"));
/// ```
pub fn address_book_path(cache_dir: &Path, network: Network) -> PathBuf {
    path_with_network(cache_dir, network).join("addressbook")
}

// Set global private permissions for created files.
//
// # Unix
// `rwxr-x---`
//
// # Windows
// TODO: does nothing.
#[cfg_attr(
    target_os = "windows",
    expect(
        clippy::missing_const_for_fn,
        reason = "remove when Windows is implemented"
    )
)]
pub fn set_private_global_file_permissions() {
    #[cfg(target_family = "unix")]
    // SAFETY: calling C.
    unsafe {
        target_os_lib::umask(0o027);
    }

    #[cfg(target_os = "windows")]
    // TODO: impl for Windows.
    {
        use target_os_lib as _;
    }
}

/// Returns the free space, in bytes, available to the current user on the filesystem
/// that holds `path`.
///
/// This uses `statvfs` on Unix and `GetDiskFreeSpaceExW` on Windows.
///
/// # Errors
///
/// This returns an error if the free space of `path` cannot be read, for example
/// if `path` does not exist, or if the platform is not supported.
pub fn free_space(path: &Path) -> std::io::Result<u64> {
    #[cfg(target_family = "unix")]
    {
        use std::{
            ffi::CString,
            io::{Error, ErrorKind},
            mem::MaybeUninit,
            os::unix::ffi::OsStrExt,
        };

        let c_path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "PATH contains a null byte"))?;

        // SAFETY: `c_path` is a valid NUL-terminated C string and `statvfs` only
        // writes to the pointer we give it, which points to a valid `statvfs`.
        let stat = unsafe {
            let mut stat = MaybeUninit::<target_os_lib::statvfs>::uninit();

            if target_os_lib::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) != 0 {
                return Err(Error::last_os_error());
            }

            stat.assume_init()
        };

        // `f_frsize` is the size of a filesystem block and `f_bavail` is the amount of
        // blocks available to unprivileged users.
        #[expect(
            clippy::useless_conversion,
            reason = "`f_bavail` and `f_frsize` are `u32` on 32-bit targets"
        )]
        let free = u64::from(stat.f_bavail) * u64::from(stat.f_frsize);

        Ok(free)
    }

    #[cfg(target_family = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;

        use target_os_lib::{core::PCWSTR, Win32::Storage::FileSystem::GetDiskFreeSpaceExW};

        let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut free = 0_u64;

        // SAFETY: `wide_path` is a NUL-terminated wide string and `free` is a valid pointer.
        let res =
            unsafe { GetDiskFreeSpaceExW(PCWSTR(wide_path.as_ptr()), Some(&mut free), None, None) };

        res.map(|()| free)
            .map_err(|e| std::io::Error::from_raw_os_error(e.code().0))
    }

    #[cfg(not(any(target_family = "unix", target_family = "windows")))]
    {
        let _ = path;

        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "reading the free space is not supported on this platform",
        ))
    }
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

    // `free_space()` should return the free space of the filesystem holding a PATH,
    // and should error when given a PATH that does not exist.
    #[test]
    fn free_space_sanity_check() {
        let free = free_space(&std::env::temp_dir()).unwrap();
        assert!(free > 0);

        assert!(free_space(Path::new("/cuprate/this/PATH/does/not/exist")).is_err());
    }
}
