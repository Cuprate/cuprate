//! Path generation functions prefixed with [`Network`].

use cuprate_types::network::Network;

/// Joins the [`Network`] to the [`Path`].
///
/// This will keep the path the same for [`Network::Mainnet`].
fn path_with_network(path: &Path, network: Network) -> PathBuf {
    match network {
        Network::Mainnet => path.to_path_buf(),
        network => path.join(network.to_string()),
    }
}

/// Create functions for creating data paths.
macro_rules! impl_data_path_with_network {
    ($(
        $(#[$attr:meta])*
        $f:ident => $path:literal
    ),* $(,)?) => {
		$(
    	    $(#[$attr])*
			pub fn $f(data_dir: &Path, network: Network) -> PathBuf {
				path_with_network(data_dir, network).join($path)
			}
		)*
    };
}

impl_data_path_with_network! {
    /// Cuprate's blockchain directory.
    blockchain_path => "blockchain",

    /// Cuprate's txpool directory.
    txpool_path => "txpool",

    /// Cuprate's logs directory.
    logs_path => "logs",

    /// Cuprate's address-book directory.
    address_book_path => "addressbook",

    /// Cuprate's arti directory.
    arti_path => "arti",
}
