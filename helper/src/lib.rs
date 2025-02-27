#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(
    clippy::should_panic_without_expect,
    clippy::single_char_lifetime_names,
    reason = "Crate-specific lints. There should be good reasoning when removing these."
)]

//---------------------------------------------------------------------------------------------------- Public API
#[cfg(feature = "asynch")]
pub mod asynch; // async collides

#[cfg(feature = "atomic")]
pub mod atomic;

#[cfg(feature = "cast")]
pub mod cast;

#[cfg(all(feature = "fs", feature = "std"))]
pub mod fs;

pub mod network;

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
//---------------------------------------------------------------------------------------------------- Private Usage

//----------------------------------------------------------------------------------------------------
