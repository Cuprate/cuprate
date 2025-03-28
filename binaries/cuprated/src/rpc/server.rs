//! The actual RPC server.

use std::net::IpAddr;

use anyhow::Error;

use crate::{config::RpcConfig, rpc::CupratedRpcHandler};

pub struct RpcServer {
    handler: CupratedRpcHandler,
    config: RpcConfig,
}

impl RpcServer {
    /// # Errors
    /// Returns error if:
    /// - The address could not be binded to
    #[expect(clippy::unnecessary_wraps)]
    pub fn new(config: RpcConfig) -> Result<Self, Error> {
        Ok(Self {
            handler: todo!(),
            config,
        })
    }
}
