#![allow(unused_crate_dependencies)]

mod block;

criterion::criterion_main! {
    block::benches,
}
