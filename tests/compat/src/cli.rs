use std::num::{NonZeroU64, NonZeroUsize};

use clap::Parser;

/// `cuprate` <-> `monerod` compatibility tester.
#[derive(Parser, Debug)]
#[command(
    about,
    long_about = None,
    long_version = format!(
        "{} {}",
        clap::crate_version!(),
        cuprate_constants::build::COMMIT
    ),
)]
pub struct Args {
    /// Base URL to use for `monerod` RPC.
    ///
    /// This must be a non-restricted RPC.
    #[arg(long, default_value_t = String::from("http://127.0.0.1:18081"))]
    pub rpc_url: String,

    /// Amount of async RPC tasks to spawn.
    #[arg(long, default_value_t = NonZeroUsize::new(4).unwrap())]
    pub rpc_tasks: NonZeroUsize,

    /// The maximum capacity of the block buffer in-between the RPC and verifier.
    ///
    /// `0` will cause the buffer to be unbounded.
    #[arg(long, default_value_t = 1000)]
    pub buffer_limit: usize,

    /// Amount of verifying threads to spawn.
    #[arg(long, default_value_t = std::thread::available_parallelism().unwrap())]
    pub threads: NonZeroUsize,

    /// Print an update every `update` amount of blocks.
    #[arg(long, default_value_t = NonZeroU64::new(500).unwrap())]
    pub update: NonZeroU64,
}

impl Args {
    pub fn get() -> Self {
        let this = Self::parse();

        println!("{this:#?}");

        this
    }
}
