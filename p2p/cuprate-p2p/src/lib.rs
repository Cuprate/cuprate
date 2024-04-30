//! Cuprate's P2P Crate.
//!
//! This crate contains a `PeerSet` which holds connected peers on a single [`NetworkZone`](monero_p2p::NetworkZone).
//! The `PeerSet` has methods to get peers by direction (inbound/outbound) or by a custom method like a load balancing
//! algorithm.
//!
//! This crate also contains the different routing methods that control how messages should be sent, i.e. broadcast to all,
//! or send to a single peer.
//!

#![allow(dead_code)]

mod client_pool;
pub mod config;
pub mod connection_maintainer;
mod constants;

pub use config::P2PConfig;
