//---------------------------------------------------------------------------------------------------- Use
use const_format::{formatcp,assertcp};
use shukusai::constants::{
	COMMIT,
	SHUKUSAI_NAME_VER,
	COLLECTION_VERSION,
	AUDIO_VERSION,
	PLAYLIST_VERSION,
	OS_ARCH,
};

//---------------------------------------------------------------------------------------------------- Version
/// The name of the Cuprate node binary.
pub const CUPRATE_BIN: &str = "cuprate";

/// `cuprate` version
///
/// This is the version of `cuprate`, the `daemon`, determined by the `Cargo.toml` 'version'.
pub const CUPRATE_VERSION: &str = {
	let version = env!("CARGO_PKG_VERSION");
	assertcp!(version.len() != 0, "CARGO_PKG_VERSION is 0 length");
	formatcp!("v{version}")
};

/// `cuprate` + version, e.g: `cuprate v0.0.1`
pub const CUPRATE_NAME_VER: &str = formatcp!("{CUPRATE_BIN} {CUPRATE_VERSION}");

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
pub const CUPRATE_BUILD_INFO: &str = formatcp!("{CUPRATE_NAME_VER}\n{OS_ARCH}\n{COMMIT}");

/// Build commit.
///
/// This needs to be set with the environment variable `COMMIT`.
/// It used to be just an `include_str!()` to the `main` branch but
/// CI running on PR branches with different branch names messes it up.
///
/// This should get set automatically in `build.rs`.
pub const CUPRATE_COMMIT: &str = env!("COMMIT");

//---------------------------------------------------------------------------------------------------- Identifiers
/// Build profile (debug/release)
///
/// This is `Debug` is `debug_assertions` is detected, else it is `Release`.
pub const CUPRATE_BUILD: &str = if cfg!(debug_assertions) { "Debug" } else { "Release" };

/// Cuprate's `dbus` connection name.
pub const CUPRATE_DBUS: &str = "com.github.Cuprate";

/// `cuprate`'s HTTP user-agent, e.g: `cuprate/v0.0.1`.
pub const CUPRATE_USER_AGENT: &str = formatcp!("{CUPRATE_BIN}/{CUPRATE_VERSION}");

/// Depends on build target, e.g:
/// - `windows x86_64`
/// - `macos aarch64`
/// - `linux x86_64`
pub const OS_ARCH: &str = formatcp!("{} {}", std::env::consts::OS, std::env::consts::ARCH);;

//---------------------------------------------------------------------------------------------------- Image
/// Cuprate's icon:
/// - `512x512`
/// - `RGBA`
/// - `PNG`
pub const CUPRATE_ICON: &[u8] = include_bytes!("../../assets/images/icon/512.png");

/// The height and width of [`CUPRATE_ICON`].
pub const CUPRATE_ICON_SIZE: u32 = 512;

//---------------------------------------------------------------------------------------------------- Directory locations
/// The name of the main project folder.
///
/// Capitalization may depend on OS (if using `disk`).
pub const CUPRATE_PROJECT_DIR: &str = "Cuprate";

/// The sub-directory where database files are saved.
pub const CUPRATE_DB_SUB_DIR: &str = "db";

/// The sub-directory where state is saved.
pub const CUPRATE_STATE_SUB_DIR: &str = "state";

/// The sub-directory for misc text files.
pub const CUPRATE_TXT_SUB_DIR: &str = "txt";

/// The sub-directory for SSL (cert/key) files.
pub const CUPRATE_SSL_SUB_DIR: &str = "ssl";

//---------------------------------------------------------------------------------------------------- Text
/// Cuprate's copyright notice.
pub const CUPRATE_COPYRIGHT: &str =
r#"Cuprate is dual-licensed under MIT/AGPL-3.0.
Its dependency tree includes many other licenses.
For more information on the project, see below:
<https://github.com/Cuprate/cuprate>"#;

//---------------------------------------------------------------------------------------------------- Network
/// Default P2P port for `cuprate`.
pub const CUPRATE_P2P_PORT: u16 = 18080;
/// Default RPC port for `cuprate`.
pub const CUPRATE_RPC_PORT: u16 = 18081;
/// Default ZMQ port for `cuprate`.
pub const CUPRATE_ZMQ_PORT: u16 = 18083;
/// Default restricted RPC port for `cuprate`.
pub const CUPRATE_RESTRICTED_RPC_PORT: u16 = 18089;

//---------------------------------------------------------------------------------------------------- Config
/// The default configuration file, as a `str`.
///
/// `cuprate` will write this to disk and use it if there is no config detected.
pub const CUPRATE_CONFIG: &str = include_str!("../config/cuprate.toml");

//---------------------------------------------------------------------------------------------------- TESTS
//#[cfg(test)]
//mod tests {
//	#[test]
//		fn __TEST__() {
//	}
//}
