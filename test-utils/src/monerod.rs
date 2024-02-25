//! Monerod Module
//!
//! This module contains a function [`monerod`] to start `monerod` - the core Monero node. Cuprate can then use
//! this to test compatibility with monerod.
//!
use std::{
    ffi::OsStr,
    fs::remove_dir_all,
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    path::PathBuf,
    process::{Child, Command, Stdio},
    str::from_utf8,
    thread::panicking,
    time::Duration,
};

use tokio::{task::yield_now, time::timeout};

mod download;

const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const MONEROD_VERSION: &str = "v0.18.3.1";
const MONEROD_STARTUP_TEXT: &str =
    "The daemon will start synchronizing with the network. This may take a long time to complete.";

const MONEROD_SHUTDOWN_TEXT: &str = "Stopping cryptonote protocol";

/// Spawns monerod and returns [`SpawnedMoneroD`].
///
/// This function will set `regtest` and the P2P/ RPC ports so these can't be included in the flags.
pub async fn monerod<T: AsRef<OsStr>>(flags: impl IntoIterator<Item = T>) -> SpawnedMoneroD {
    let path_to_monerod = download::check_download_monerod().await.unwrap();

    let rpc_port = get_available_port(&[]);
    let p2p_port = get_available_port(&[rpc_port]);
    let zmq_port = get_available_port(&[rpc_port, p2p_port]);

    let db_location =
        download::find_target().join(format!("monerod_data_{}", rand::random::<u64>()));

    // TODO: set a random DB location
    let mut monerod = Command::new(path_to_monerod)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(flags)
        .arg("--regtest")
        .arg("--log-level=2")
        .arg(format!("--p2p-bind-port={}", p2p_port))
        .arg(format!("--rpc-bind-port={}", rpc_port))
        .arg(format!("--zmq-rpc-bind-port={}", zmq_port))
        .arg(format!("--data-dir={}", db_location.display()))
        .arg("--non-interactive")
        .spawn()
        .unwrap();

    let mut logs = String::new();

    timeout(Duration::from_secs(30), async {
        loop {
            let mut next_str = [0];
            let _ = monerod
                .stdout
                .as_mut()
                .unwrap()
                .read(&mut next_str)
                .unwrap();

            logs.push_str(from_utf8(&next_str).unwrap());

            if logs.contains(MONEROD_SHUTDOWN_TEXT) {
                panic!("Failed to start monerod, logs: \n {logs}");
            }

            if logs.contains(MONEROD_STARTUP_TEXT) {
                break;
            }
            // this is blocking code but as this is for tests performance isn't a priority. However we should still yield so
            // the timeout works.
            yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("Failed to start monerod in time, logs: {logs}"));

    SpawnedMoneroD {
        process: monerod,
        rpc_port,
        p2p_port,
        data_dir: db_location,
        start_up_logs: logs,
    }
}

fn get_available_port(already_taken: &[u16]) -> u16 {
    loop {
        // Using `0` makes the OS return a random available port.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        if !already_taken.contains(&port) {
            return port;
        }
    }
}

/// A struct representing a spawned monerod.
pub struct SpawnedMoneroD {
    /// A handle to the monerod process, monerod will be stopped when this is dropped.
    process: Child,
    /// The RPC port of the monerod instance.
    rpc_port: u16,
    /// The P2P port of the monerod instance.
    p2p_port: u16,
    /// The Path to the monerod data-dir
    data_dir: PathBuf,
    /// The logs upto [`MONEROD_STARTUP_TEXT`].
    start_up_logs: String,
}

impl SpawnedMoneroD {
    /// Returns the p2p port of the spawned monerod
    pub fn p2p_addr(&self) -> SocketAddr {
        SocketAddr::new(LOCALHOST, self.p2p_port)
    }

    /// Returns the RPC port of the spawned monerod
    pub fn rpc_port(&self) -> SocketAddr {
        SocketAddr::new(LOCALHOST, self.rpc_port)
    }
}

impl Drop for SpawnedMoneroD {
    fn drop(&mut self) {
        let mut error = false;

        if self.process.kill().is_err() {
            error = true;
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
            println!("{}{out}", self.start_up_logs);
            println!("------END-MONEROD-LOGS------");
        }

        if let Err(e) = remove_dir_all(&self.data_dir) {
            error = true;
            println!(
                "Could not remove monerod data-dir at: {:?}, error: {}",
                self.data_dir, e
            )
        }

        if error && !panicking() {
            // `println` only outputs in a test when panicking so if there is an error while
            // dropping monerod but not an error in the test then we need to panic to make sure
            // the println!s are output.
            panic!("Error while dropping monerod");
        }
    }
}
