//---------------------------------------------------------------------------------------------------- Use
use const_format::{formatcp,assertcp};

//---------------------------------------------------------------------------------------------------- Version
/// The name of the project and main directory.
pub const CUPRATE: &str = "Cuprate";

/// The name of the Cuprate node binary.
pub const BIN: &str = "cuprate";

/// `cuprate` version
///
/// This is the version of `cuprate`, the `daemon`, determined by the `Cargo.toml` 'version'.
pub const VERSION: &str = {
	const V: &str = env!("CARGO_PKG_VERSION");
	assertcp!(V.len() != 0, "CARGO_PKG_VERSION is 0 length");
	formatcp!("v{V}")
};

/// `cuprate` + version, e.g: `cuprate v0.0.1`
pub const NAME_VER: &str = formatcp!("{BIN} {VERSION}");

/// - cuprate name + version
/// - OS + Arch
/// - Git commit hash
///
/// e.g:
///
/// ```
/// cuprate v1.0.0
/// windows x86_64
/// 34dd105a0c585dc34ba0a1ace625663bde00a1dc
/// ```
pub const BUILD_INFO: &str = formatcp!("{NAME_VER}\n{OS_ARCH}\n{COMMIT}");

/// Build commit.
///
/// This needs to be set with the environment variable `COMMIT`.
/// It used to be just an `include_str!()` to the `main` branch but
/// CI running on PR branches with different branch names messes it up.
///
/// This should get set automatically in `build.rs`.
pub const COMMIT: &str = env!("COMMIT");

//---------------------------------------------------------------------------------------------------- Identifiers
/// Build profile (debug/release)
///
/// This is `Debug` is `debug_assertions` is detected, else it is `Release`.
pub const BUILD: &str = if cfg!(debug_assertions) { "Debug" } else { "Release" };

/// Cuprate's `dbus` connection name.
pub const DBUS: &str = "com.github.Cuprate";

/// `cuprate`'s HTTP user-agent, e.g: `cuprate/v0.0.1`.
pub const USER_AGENT: &str = formatcp!("{BIN}/{VERSION}");

/// Depends on build target, e.g:
/// - `windows x86_64`
/// - `macos aarch64`
/// - `linux x86_64`
pub const OS_ARCH: &str = formatcp!("{} {}", std::env::consts::OS, std::env::consts::ARCH);

//---------------------------------------------------------------------------------------------------- Image
/// Cuprate's icon:
/// - `512x512`
/// - `RGBA`
/// - `PNG`
pub const ICON: &[u8] = todo!(); // include_bytes!("../../assets/images/icon/512.png");

/// The height and width of [`ICON`].
pub const ICON_SIZE: u32 = 512;

//---------------------------------------------------------------------------------------------------- Directory locations
/// The name of the main project folder.
///
/// Capitalization may depend on OS (if using `disk`).
pub const PROJECT_DIR: &str = "Cuprate";

/// The sub-directory where database files are saved.
pub const DB_SUB_DIR: &str = "db";

/// The sub-directory where state is saved.
pub const STATE_SUB_DIR: &str = "state";

/// The sub-directory for misc text files.
pub const TXT_SUB_DIR: &str = "txt";

/// The sub-directory for SSL (cert/key) files.
pub const SSL_SUB_DIR: &str = "ssl";

//---------------------------------------------------------------------------------------------------- Text
/// Cuprate's copyright notice.
pub const COPYRIGHT: &str =
r#"Cuprate is dual-licensed under MIT/AGPL-3.0.
Its dependency tree includes many other licenses.
For more information on the project, see below:
<https://github.com/Cuprate/cuprate>"#;

//---------------------------------------------------------------------------------------------------- Network
/// Default P2P port for `cuprate`.
pub const DEFAULT_P2P_PORT: u16 = 18080;
/// Default RPC port for `cuprate`.
pub const DEFAULT_RPC_PORT: u16 = 18081;
/// Default ZMQ port for `cuprate`.
pub const DEFAULT_ZMQ_PORT: u16 = 18083;
/// Default restricted RPC port for `cuprate`.
pub const DEFAULT_RESTRICTED_RPC_PORT: u16 = 18089;

//---------------------------------------------------------------------------------------------------- Config
/// The default configuration file, as a `str`.
///
/// `cuprate` will write this to disk and use it if there is no config detected.
pub const DEFAULT_CONFIG: &str = todo!(); // include_str!(formatcp!("../config/{VERSION}.toml"));

//---------------------------------------------------------------------------------------------------- TESTS
//#[cfg(test)]
//mod tests {
//	#[test]
//		fn __TEST__() {
//	}
//}
