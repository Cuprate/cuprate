#![expect(unused_crate_dependencies)]

mod db;
mod env;
mod storable;

criterion::criterion_main! {
    db::benches,
    env::benches,
    storable::benches,
}
