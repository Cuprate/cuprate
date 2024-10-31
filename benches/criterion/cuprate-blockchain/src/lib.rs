#![allow(unused_crate_dependencies, reason = "used in benchmarks")]

mod blocks;
mod tmp_env;

pub use blocks::generate_fake_blocks;
pub use tmp_env::TmpEnv;
