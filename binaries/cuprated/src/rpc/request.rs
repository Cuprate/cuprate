//! Convenience functions for requests/responses.
//!
//! This module implements many methods for
//! [`CupratedRpcHandler`](crate::rpc::CupratedRpcHandler)
//! that are simple wrappers around the request/response API provided
//! by the multiple [`tower::Service`]s.
//!
//! These exist to prevent noise like `unreachable!()`
//! from being everywhere in the actual handler functions.
//!
//! Each module implements methods for a specific API, e.g.
//! the [`blockchain`] modules contains methods for the
//! blockchain database [`tower::Service`] API.

mod address_book;
mod blockchain;
mod blockchain_context;
mod blockchain_manager;
mod txpool;
