//---------------------------------------------------------------------------------------------------- Use
use zeroize::Zeroize;
use anyhow::anyhow;
use tokio_native_tls::{
	TlsAcceptor,
	native_tls::{
		Identity,
		TlsAcceptor as TlsAcceptorNative,
	},
};
use std::path::{
	Path,PathBuf,
};

//----------------------------------------------------------------------------------------------------
pub fn get_tls_acceptor(path_cert: &Path, path_key: &Path) -> Result<&'static TlsAcceptor, anyhow::Error> {
	// Read cert.
	let mut cert = std::fs::read(path_cert)?;

	// Read key.
	let mut key = std::fs::read(path_key)?;

	let acceptor = TlsAcceptor::from(build_cert(cert, key)?);
	let acceptor: &'static TlsAcceptor = Box::leak(Box::new(acceptor));

	Ok(acceptor)
}

fn build_cert(mut cert: Vec<u8>, mut key: Vec<u8>) -> Result<TlsAcceptorNative, anyhow::Error> {
	// Build.
	let identity = Identity::from_pkcs8(&cert, &key)?;
	let acceptor = TlsAcceptorNative::builder(identity).build()?;

	// Zeroize cert + key.
	cert.zeroize();
	key.zeroize();

	Ok(acceptor)
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
	use super::*;

	const CERT: &[u8] = include_bytes!("../../assets/tls/cert.pem");
	const KEY: &[u8] = include_bytes!("../../assets/tls/key.pem");

	#[test]
	fn cert() {
		build_cert(CERT.into(), KEY.into()).unwrap();
	}
}
