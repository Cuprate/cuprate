//! RPC server initialization and main loop.

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use anyhow::Error;
use tokio::net::TcpListener;
use tower::limit::rate::RateLimitLayer;
use tower_http::{
    compression::CompressionLayer, decompression::DecompressionLayer, limit::RequestBodyLimitLayer,
};
use tracing::{info, warn};

use cuprate_rpc_interface::{RouterBuilder, RpcHandlerDummy};

use crate::{
    config::{RpcConfig, SharedRpcConfig},
    rpc::CupratedRpcHandler,
};

/// Initialize the RPC server(s).
///
/// # Panics
/// This function will panic if the server(s) could not be started.
pub fn init_rpc_servers(config: RpcConfig) {
    for (c, restricted) in [(config.unrestricted, false), (config.restricted, true)] {
        let Some(socket_addr) = c.address else {
            info!("Skipping RPC server (restricted={restricted})");
            continue;
        };

        tokio::task::spawn(async move {
            run_rpc_server(
                restricted,
                socket_addr,
                c,
                config.i_know_what_im_doing_allow_public_unrestricted_rpc,
            )
            .await
            .unwrap();
        });
    }
}

/// This initializes and runs an RPC server.
///
/// The function will only return when the server itself returns.
async fn run_rpc_server(
    restricted: bool,
    socket_addr: SocketAddr,
    config: SharedRpcConfig,
    i_know_what_im_doing_allow_public_unrestricted_rpc: bool,
) -> Result<(), Error> {
    info!("Starting RPC server (restricted={restricted}) on {socket_addr}");

    if !restricted {
        // FIXME: more accurate detection on IP local-ness.
        // <https://github.com/rust-lang/rust/issues/27709>
        let is_local = match socket_addr.ip() {
            IpAddr::V4(ip) => ip.is_loopback() || ip.is_private(),
            IpAddr::V6(ip) => {
                ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local()
            }
        };

        if !is_local {
            if i_know_what_im_doing_allow_public_unrestricted_rpc {
                warn!("Starting an unrestricted RPC server to a non-local address ({socket_addr}), this is dangerous!");
            } else {
                panic!("Refusing to start an unrestricted RPC server on a non-local address ({socket_addr})");
            }
        }
    }

    // Create the router.
    //
    // TODO: impl more layers, rate-limiting, configuration, etc.
    let state = RpcHandlerDummy { restricted };
    // TODO:
    // - add functions that are `all()` but for restricted RPC
    // - enable aliases automatically `other_get_height` + `other_getheight`?
    //
    // FIXME:
    // - `json_rpc` is 1 endpoint; `RouterBuilder` operates at the
    //   level endpoint; we can't selectively enable certain `json_rpc` methods
    let router = RouterBuilder::new().fallback().build().with_state(state);

    // Enable request (de)compression.
    let router = if config.gzip || config.br {
        router
            .layer(DecompressionLayer::new().gzip(config.gzip).br(config.br))
            .layer(CompressionLayer::new().gzip(config.gzip).br(config.br))
    } else {
        router
    };

    // Add restrictive layers if restricted RPC.
    //
    // TODO: <https://github.com/Cuprate/cuprate/issues/445>
    let router = if config.request_byte_limit != 0 {
        router.layer(RequestBodyLimitLayer::new(config.request_byte_limit))
    } else {
        router
    };

    // Start the server.
    //
    // TODO: impl custom server code, don't use axum.
    let listener = TcpListener::bind(socket_addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
