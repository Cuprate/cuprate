#![allow(unused_crate_dependencies, reason = "used in benchmarks")]

mod tmp_env;
mod blocks;

pub use blocks::generate_fake_blocks;
pub use tmp_env::TmpEnv;
