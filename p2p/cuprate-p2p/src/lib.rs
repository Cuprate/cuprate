//! Cuprate's P2P Crate.
//!
//! This crate contains a [`ClientPool`](client_pool::ClientPool) which holds connected peers on a single [`NetworkZone`](monero_p2p::NetworkZone).
//!
//! This crate also contains the different routing methods that control how messages should be sent, i.e. broadcast to all,
//! or send to a single peer.
//!
#![allow(dead_code)]

pub mod client_pool;
pub mod config;
pub mod connection_maintainer;
mod constants;
mod sync_states;

pub use config::P2PConfig;
