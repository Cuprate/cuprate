use std::{process::exit, time::SystemTime};

use clap::Parser;

use crate::{
    api::GithubApiClient, changelog::generate_changelog, crates::CuprateCrates,
    free::generate_cuprated_help_text,
};

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// CLI arguments.
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
pub struct Cli {
    /// List all Cuprate crates and their versions.
    #[arg(long)]
    pub list_crates: bool,

    /// The start UNIX timestamp of the changelog.
    #[arg(long, default_value_t)]
    pub start_timestamp: u64,

    /// The end UNIX timestamp of the changelog.
    #[arg(long, default_value_t = current_unix_timestamp())]
    pub end_timestamp: u64,

    /// The release's code name (should be a metal).
    #[arg(long)]
    pub release_name: Option<String>,

    /// Generate and output the changelog to stdout.
    #[arg(long)]
    pub changelog: bool,

    /// Output `cuprated --help` to stdout.
    #[arg(long)]
    pub cuprated_help: bool,
}

impl Cli {
    /// Complete any quick requests asked for in [`Cli`].
    pub fn do_quick_requests(self) -> Self {
        let crates = CuprateCrates::new();
        let api = GithubApiClient::new(self.start_timestamp, self.end_timestamp);

        if self.list_crates {
            for pkg in crates.packages {
                println!("{} {}", pkg.version, pkg.name);
            }
            exit(0);
        }

        if self.changelog {
            println!("{}", generate_changelog(crates, api, self.release_name));
            exit(0);
        }

        if self.cuprated_help {
            println!("{}", generate_cuprated_help_text());
            exit(0);
        }

        self
    }

    pub fn init() -> Self {
        let this = Self::parse();
        this.do_quick_requests()
    }
}
