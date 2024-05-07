//! Monero RPC client.
//!
//! This module is a client for Monero RPC that maps the types
//! into the native types used by Cuprate found in `cuprate_types`.
//!
//! # Usage
//! ```rust,ignore
//! #[tokio::main]
//! async fn main() {
//!     // Create RPC client.
//!     let rpc = HttpRpcClient::new(None).await;
//!
//!     // Collect 20 blocks.
//!     let mut vec: Vec<VerifiedBlockInformation> = vec![];
//!     for height in (3130269 - 20)..3130269 {
//!         vec.push(rpc.get_verified_block_information(height).await);
//!     }
//! }
//! ```

mod client;
pub use client::HttpRpcClient;

mod constants;
pub use constants::LOCALHOST_RPC_URL;
