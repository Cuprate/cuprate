//! # Dandelion Tower
//!
//! This crate implements [dandelion++](https://arxiv.org/pdf/1805.11060.pdf), using [`tower`].
//!  
//! This crate provides 2 [`tower::Service`]s, a [`DandelionRouter`] and a [`DandelionPool`](pool::DandelionPool).
//! The router is pretty minimal and only handles the absolute necessary data to route transactions, whereas the
//! pool keeps track of all data necessary for dandelion++ but requires you to provide a backing tx-pool.
//!
//! This split was done not because the [`DandelionPool`](pool::DandelionPool) is unnecessary but because it is hard
//! to cover a wide range of projects when abstracting over the tx-pool. Not using the [`DandelionPool`](pool::DandelionPool)
//! requires you to implement part of the paper yourself.
//!
//! # Features
//!
//! This crate only has one feature `txpool` which enables [`DandelionPool`](pool::DandelionPool).
//!
//! # Needed Services
//!
//! To use this crate you need to provide a few types.
//!
//! ## Diffuse Service
//!
//! This service should implement diffusion, which is sending the transaction to every peer, with each peer
//! having a timer using the exponential distribution and batch sending all txs that were queued in that time.
//!
//! The diffuse service should have a request of [`DiffuseRequest`](traits::DiffuseRequest) and it's error
//! should be [`tower::BoxError`].
//!
//! ## Outbound Peer TryStream
//!
//! The outbound peer [`TryStream`](futures::TryStream) should provide a stream of randomly selected outbound
//! peers, these peers will then be used to route stem txs to.
//!
//! The peers will not be returned anywhere, so it is recommended to wrap them in some sort of drop guard that returns
//! them back to a peer set.
//!
//! ## Peer Service
//!
//! This service represents a connection to an individual peer, this should be returned from the Outbound Peer
//! TryStream. This should immediately send the transaction to the peer when requested, it should _not_ set
//! a timer.
//!
//! The peer service should have a request of [`StemRequest`](traits::StemRequest) and its error
//! should be [`tower::BoxError`].
//!
//! ## Backing Pool
//!
//! ([`DandelionPool`](pool::DandelionPool) only)
//!
//! This service is a backing tx-pool, in memory or on disk.
//! The backing pool should have a request of [`TxStoreRequest`](traits::TxStoreRequest) and a response of
//! [`TxStoreResponse`](traits::TxStoreResponse), with an error of [`tower::BoxError`].
//!
//! Users should keep a handle to the backing pool to request data from it, when requesting data you _must_
//! make sure you only look in the public pool if you are going to be giving data to peers, as stem transactions
//! must stay private.
//!
//! When removing data, for example because of a new block, you can remove from both pools provided it doesn't leak
//! any data about stem transactions. You will probably want to set up a task that monitors the tx pool for stuck transactions,
//! transactions that slipped in just as one was removed etc, this crate does not handle that.
mod config;
#[cfg(feature = "txpool")]
pub mod pool;
mod router;
#[cfg(test)]
mod tests;
pub mod traits;

pub use config::*;
pub use router::*;
