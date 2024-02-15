//! Monerod Module
//!
//! This module contains a function [`monerod`] to start `monerod` - the core Monero node. Cuprate can then use
//! this to test compatibility with monerod.
//!
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    process::Stdio,
    sync::OnceLock,
    time::Duration,
};

use rand::Rng;
use tokio::{
    process::{Child, Command},
    sync::{mpsc, oneshot},
};

mod download;

const LOCAL_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const MONEROD_VERSION: &str = "v0.18.3.1";

#[allow(clippy::type_complexity)]
static MONEROD_HANDLER_CHANNEL: OnceLock<
    mpsc::Sender<(MoneroDRequest, oneshot::Sender<(SocketAddr, SocketAddr)>)>,
> = OnceLock::new();

/// Spawns monerod and returns the p2p address and rpc address.
///
/// When spawning monerod, this module will try to use an already spawned instance to reduce the amount
/// of instances that need to be spawned.
///
/// This function will set `regtest` and the P2P/ RPC ports so these can't be included in the flags.
pub async fn monerod(flags: Vec<String>, mutable: bool) -> (SocketAddr, SocketAddr) {
    // TODO: sort flags so the same flags in a different order will give the same monerod?

    // We only actually need these channels on first run so this might be wasteful
    let (tx, rx) = mpsc::channel(3);
    let mut should_spawn = false;

    let monero_handler_tx = MONEROD_HANDLER_CHANNEL.get_or_init(|| {
        should_spawn = true;
        tx
    });

    if should_spawn {
        // If this call was the first call to start a monerod instance then start the handler.
        let manager = MoneroDManager::new().await;
        tokio::task::spawn(manager.run(rx));
    }

    let (tx, rx) = oneshot::channel();

    monero_handler_tx
        .send((MoneroDRequest { mutable, flags }, tx))
        .await
        .unwrap();

    // Give monerod some time to start
    tokio::time::sleep(Duration::from_secs(5)).await;
    rx.await.unwrap()
}

/// A request sent to get an address to a monerod instance.
struct MoneroDRequest {
    /// Whether we plan to change the state of the spawned monerod's blockchain.
    mutable: bool,
    /// Start flags to start monerod with.
    flags: Vec<String>,
}

/// A struct representing a spawned monerod.
struct SpawnedMoneroD {
    /// A marker for if the test that spawned this monerod is going to mutate it.
    mutable: bool,
    /// A handle to the monerod process, monerod will be stopped when this is dropped.
    #[allow(dead_code)]
    process: Child,
    /// The RPC port of the monerod instance.
    rpc_port: u16,
    /// The P2P port of the monerod instance.
    p2p_port: u16,
}

/// A manger of spawned monerods.
struct MoneroDManager {
    /// A map of start flags to monerods.
    monerods: HashMap<Vec<String>, Vec<SpawnedMoneroD>>,
    /// The path to the monerod binary.
    path_to_monerod: PathBuf,
}

impl MoneroDManager {
    pub async fn new() -> Self {
        let path_to_monerod = download::check_download_monerod().await.unwrap();

        Self {
            monerods: Default::default(),
            path_to_monerod,
        }
    }

    pub async fn run(
        mut self,
        mut rx: mpsc::Receiver<(MoneroDRequest, oneshot::Sender<(SocketAddr, SocketAddr)>)>,
    ) {
        while let Some((req, tx)) = rx.recv().await {
            let (p2p_port, rpc_port) = self.get_monerod_with_flags(req.flags, req.mutable);
            let _ = tx.send((
                SocketAddr::new(LOCAL_HOST, p2p_port),
                SocketAddr::new(LOCAL_HOST, rpc_port),
            ));
        }
    }

    /// Tries to get a current monerod instance or spans one if there is not an appropriate one to use.
    /// Returns the p2p port and then the RPC port of the spawned monerd.
    fn get_monerod_with_flags(&mut self, flags: Vec<String>, mutable: bool) -> (u16, u16) {
        // If we need to mutate monerod's blockchain then we can't reuse one.
        if !mutable {
            if let Some(monerods) = &self.monerods.get(&flags) {
                for monerod in monerods.iter() {
                    if !monerod.mutable {
                        return (monerod.p2p_port, monerod.rpc_port);
                    }
                }
            }
        }

        let mut rng = rand::thread_rng();
        // Use random ports and *hope* we don't get a collision (TODO: just keep a counter and increment?)
        let rpc_port: u16 = rng.gen_range(1500..u16::MAX);
        let p2p_port: u16 = rng.gen_range(1500..u16::MAX);

        // TODO: set a different DB location per node
        let monerod = Command::new(&self.path_to_monerod)
            .stdout(Stdio::null())
            .stdin(Stdio::piped())
            .args(&flags)
            .arg("--regtest")
            .arg(format!("--p2p-bind-port={}", p2p_port))
            .arg(format!("--rpc-bind-port={}", rpc_port))
            .kill_on_drop(true)
            .spawn()
            .unwrap();

        let spawned_monerod = SpawnedMoneroD {
            mutable,
            process: monerod,
            rpc_port,
            p2p_port,
        };

        self.monerods
            .entry(flags.clone())
            .or_default()
            .push(spawned_monerod);
        let Some(monerods) = self.monerods.get(&flags) else {
            unreachable!()
        };

        for monerod in monerods {
            if !monerod.mutable {
                return (monerod.p2p_port, monerod.rpc_port);
            }
        }
        unreachable!()
    }
}
