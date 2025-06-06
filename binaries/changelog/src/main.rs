#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(unreachable_pub, reason = "Binary")]
#![allow(clippy::needless_pass_by_value, reason = "Efficiency doesn't matter")]

mod api;
mod changelog;
mod cli;
mod crates;
mod free;

fn main() {
    free::assert_repo_root();

    cli::Cli::init();
}
