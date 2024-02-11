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
pub async fn monerod(flags: Vec<String>, mutable: bool) -> (SocketAddr, SocketAddr) {
    // TODO: sort flags so the same flags in a different order will give the same monerod?

    // We only actually need these channels on first run so this might be waste full
    let (tx, rx) = mpsc::channel(3);
    let mut should_spwan = false;

    let monero_handler_tx = MONEROD_HANDLER_CHANNEL.get_or_init(|| {
        should_spwan = true;
        tx
    });

    if should_spwan {
        let manager = MoneroDManager::new().await;
        tokio::task::spawn(manager.run(rx));
    }

    let (tx, rx) = oneshot::channel();

    monero_handler_tx
        .send((MoneroDRequest { mutable, flags }, tx))
        .await
        .unwrap();

    // Give monerod some time to start
    tokio::time::sleep(Duration::from_secs(3)).await;
    rx.await.unwrap()
}

struct MoneroDRequest {
    mutable: bool,
    flags: Vec<String>,
}

#[allow(dead_code)]
struct SpwanedMoneroD {
    /// A marker for if the test that spawned this monerod is going to mutate it.
    mutable: bool,
    process: Child,
    rpc_port: u16,
    p2p_port: u16,
}

struct MoneroDManager {
    /// A map of start flags to monerods
    monerods: HashMap<Vec<String>, Vec<SpwanedMoneroD>>,

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

    fn get_monerod_with_flags(&mut self, flags: Vec<String>, mutable: bool) -> (u16, u16) {
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

        let rpc_port: u16 = rng.gen_range(1500..u16::MAX);
        let p2p_port: u16 = rng.gen_range(1500..u16::MAX);

        // TODO: set a different DB location per node
        let monerod = Command::new(&self.path_to_monerod)
            .stdout(Stdio::null())
            .args(&flags)
            .arg("--regtest")
            .arg(format!("--p2p-bind-port={}", p2p_port))
            .arg(format!("--rpc-bind-port={}", rpc_port))
            .kill_on_drop(true)
            .spawn()
            .unwrap();

        let spawned_monerd = SpwanedMoneroD {
            mutable,
            process: monerod,
            rpc_port,
            p2p_port,
        };

        self.monerods
            .entry(flags.clone())
            .or_default()
            .push(spawned_monerd);
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
