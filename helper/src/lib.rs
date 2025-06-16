#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

//---------------------------------------------------------------------------------------------------- Public API
#[cfg(feature = "asynch")]
pub mod asynch; // async collides

#[cfg(feature = "cast")]
pub mod cast;

#[cfg(feature = "num")]
pub mod num;

#[cfg(feature = "map")]
pub mod map;

#[cfg(feature = "thread")]
pub mod thread;

#[cfg(feature = "time")]
pub mod time;

#[cfg(feature = "tx")]
pub mod tx;

#[cfg(feature = "crypto")]
pub mod crypto;

#[cfg(feature = "fmt")]
pub mod fmt;

#[cfg(feature = "net")]
pub mod net;
