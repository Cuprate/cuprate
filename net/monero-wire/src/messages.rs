// Rust Levin Library
// Written in 2023 by
//   Cuprate Contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//

//! This module defines a Monero `Message` enum which contains
//! every possible Monero network message (levin body)

pub mod admin;
pub mod common;
pub mod protocol;

use std::fmt::Debug;

pub use common::{BasicNodeData, CoreSyncData, PeerID, PeerListEntryBase};
pub use admin::{Handshake, TimedSync, Ping, SupportFlags};
pub use protocol::{
    NewBlock, NewTransactions, GetObjectsRequest, GetObjectsResponse, ChainRequest, ChainResponse, NewFluffyBlock,
    FluffyMissingTransactionsRequest, GetTxPoolCompliment,
};

fn zero_val<T: From<u8>>() -> T {
    T::from(0_u8)
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

pub trait Message: Sized {
    type EncodingError: Debug;
    fn decode(buf: &[u8]) -> Result<Self, Self::EncodingError>;
    fn encode(&self) -> Result<Vec<u8>, Self::EncodingError>;
}

pub trait AdminMessage {
    const ID: u32;
    type Request: Message;
    type Response: Message;
}

pub trait ProtocolMessage {
    const ID: u32;
    type Notification: Message;
}

macro_rules! p2p_command {
    ($($message:ident),+) => {
        pub enum P2pCommand {
            $($message,)+
        }

        pub struct P2pCommandFromU32Err;
        impl TryFrom<u32> for P2pCommand {
            type Error = P2pCommandFromU32Err;
            fn try_from(value: u32) -> Result<Self, Self::Error> {
                match value {
                    $($message::ID => Ok(P2pCommand::$message),)+
                    _ => Err(P2pCommandFromU32Err)
                }
            }
        }
        impl From<P2pCommand> for u32 {
            fn from(val: P2pCommand) -> Self {
                match val {
                    $(P2pCommand::$message => $message::ID,)+
                }
            }
        }
    };
}

p2p_command!(
    Handshake,
    TimedSync,
    Ping,
    SupportFlags,
    NewBlock,
    NewTransactions,
    GetObjectsRequest,
    GetObjectsResponse,
    ChainRequest,
    ChainResponse,
    NewFluffyBlock,
    FluffyMissingTransactionsRequest,
    GetTxPoolCompliment
);
