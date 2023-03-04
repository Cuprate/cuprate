//! # Monero Wire
//!
//! A crate defining Monero network messages and network addresses,
//! built on top of the levin crate.
//!
//! ## License
//!
//! This project is licensed under the MIT License.

// Coding conventions
#![forbid(unsafe_code)]
#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(unused_mut)]
#![deny(missing_docs)]

#[macro_use]
mod internal_macros;
pub mod messages;
pub mod network_address;

pub use network_address::NetworkAddress;

// re-exports
pub use levin;
pub use levin::message_sink::MessageSink;
pub use levin::message_stream::MessageStream;

use levin::BucketError;

/// The possible commands that can be in a levin header
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum P2pCommand {
    // 100* commands
    /// Handshake
    Handshake,
    /// TimedSync
    TimedSync,
    /// Ping
    Ping,
    /// SupportFlags
    SupportFlags,

    // 200* commands
    /// NewBlock
    NewBlock,
    /// NewTransactions
    NewTransactions,
    /// RequestGetObject
    RequestGetObject,
    /// ResponseGetObject
    ResponseGetObject,
    /// RequestChain
    RequestChain,
    /// ResponseChainEntry
    ResponseChainEntry,
    /// NewFluffyBlock
    NewFluffyBlock,
    /// RequestFluffyMissingTx
    RequestFluffyMissingTx,
    /// GetTxPoolComplement
    GetTxPoolComplement,
}

impl TryFrom<u32> for P2pCommand {
    type Error = BucketError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1001 => Ok(P2pCommand::Handshake),
            1002 => Ok(P2pCommand::TimedSync),
            1003 => Ok(P2pCommand::Ping),
            1007 => Ok(P2pCommand::SupportFlags),

            2001 => Ok(P2pCommand::NewBlock),
            2002 => Ok(P2pCommand::NewTransactions),
            2003 => Ok(P2pCommand::RequestGetObject),
            2004 => Ok(P2pCommand::ResponseGetObject),
            2006 => Ok(P2pCommand::RequestChain),
            2007 => Ok(P2pCommand::ResponseChainEntry),
            2008 => Ok(P2pCommand::NewFluffyBlock),
            2009 => Ok(P2pCommand::RequestFluffyMissingTx),
            2010 => Ok(P2pCommand::GetTxPoolComplement),

            _ => Err(BucketError::UnsupportedP2pCommand(value)),
        }
    }
}

impl From<P2pCommand> for u32 {
    fn from(val: P2pCommand) -> Self {
        match val {
            P2pCommand::Handshake => 1001,
            P2pCommand::TimedSync => 1002,
            P2pCommand::Ping => 1003,
            P2pCommand::SupportFlags => 1007,

            P2pCommand::NewBlock => 2001,
            P2pCommand::NewTransactions => 2002,
            P2pCommand::RequestGetObject => 2003,
            P2pCommand::ResponseGetObject => 2004,
            P2pCommand::RequestChain => 2006,
            P2pCommand::ResponseChainEntry => 2007,
            P2pCommand::NewFluffyBlock => 2008,
            P2pCommand::RequestFluffyMissingTx => 2009,
            P2pCommand::GetTxPoolComplement => 2010,
        }
    }
}
