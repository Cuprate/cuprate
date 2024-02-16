//! Monerod Module
//!
//! This module contains a function [`monerod`] to start `monerod` - the core Monero node. Cuprate can then use
//! this to test compatibility with monerod.
//!
use std::{
    ffi::OsStr,
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    process::{Child, Command, Stdio},
    thread::panicking,
    time::Duration,
};

use rand::Rng;
use tokio::time::sleep;

mod download;

const LOCAL_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const MONEROD_VERSION: &str = "v0.18.3.1";

/// Spawns monerod and returns [`SpawnedMoneroD`].
///
/// This function will set `regtest` and the P2P/ RPC ports so these can't be included in the flags.
pub async fn monerod<T: AsRef<OsStr>>(flags: impl IntoIterator<Item = T>) -> SpawnedMoneroD {
    let path_to_monerod = download::check_download_monerod().await.unwrap();
    let mut rng = rand::thread_rng();

    // Use random ports and *hope* we don't get a collision (TODO: just keep an atomic counter and increment?)
    let rpc_port: u16 = rng.gen_range(1500..u16::MAX);
    let p2p_port: u16 = rng.gen_range(1500..u16::MAX);

    // TODO: set a random DB location &   zMQ port
    let monerod = Command::new(path_to_monerod)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .args(flags)
        .arg("--regtest")
        .arg("--log-level=2")
        .arg(format!("--p2p-bind-port={}", p2p_port))
        .arg(format!("--rpc-bind-port={}", rpc_port))
        .spawn()
        .unwrap();

    sleep(Duration::from_secs(3)).await;

    SpawnedMoneroD {
        process: monerod,
        rpc_port,
        p2p_port,
    }
}

/// A struct representing a spawned monerod.
pub struct SpawnedMoneroD {
    /// A handle to the monerod process, monerod will be stopped when this is dropped.
    #[allow(dead_code)]
    process: Child,
    /// The RPC port of the monerod instance.
    rpc_port: u16,
    /// The P2P port of the monerod instance.
    p2p_port: u16,
}

impl SpawnedMoneroD {
    /// Returns the p2p port of the spawned monerod
    pub fn p2p_addr(&self) -> SocketAddr {
        SocketAddr::new(LOCAL_HOST, self.p2p_port)
    }

    /// Returns the RPC port of the spawned monerod
    pub fn rpc_port(&self) -> SocketAddr {
        SocketAddr::new(LOCAL_HOST, self.rpc_port)
    }
}

impl Drop for SpawnedMoneroD {
    fn drop(&mut self) {
        if self.process.kill().is_err() {
            println!("Failed to kill monerod, process id: {}", self.process.id())
        }

        if panicking() {
            // If we are panicking then a test failed so print monerod's logs.

            let mut out = String::new();

            if self
                .process
                .stdout
                .as_mut()
                .unwrap()
                .read_to_string(&mut out)
                .is_err()
            {
                println!("Failed to get monerod's logs.");
            }

            println!("-----START-MONEROD-LOGS-----");
            println!("{out}",);
            println!("------END-MONEROD-LOGS------");
        }
    }
}
