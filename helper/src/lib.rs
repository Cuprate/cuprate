#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

//---------------------------------------------------------------------------------------------------- Public API
#[cfg(feature = "asynch")]
pub mod asynch; // async collides

#[cfg(feature = "atomic")]
pub mod atomic;

#[cfg(feature = "cast")]
pub mod cast;

#[cfg(feature = "constants")]
pub mod constants;

#[cfg(feature = "fs")]
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
//---------------------------------------------------------------------------------------------------- Private Usage

//----------------------------------------------------------------------------------------------------
