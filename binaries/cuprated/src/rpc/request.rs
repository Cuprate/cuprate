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

pub(super) mod address_book;
pub(super) mod blockchain;
pub(super) mod blockchain_context;
pub(super) mod blockchain_manager;
pub(super) mod txpool;
