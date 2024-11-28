//! Benchmarks for [`Response`].
#![allow(unused_attributes, unused_crate_dependencies, dropping_copy_types)]

use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use function_name::named;

use cuprate_cryptonight::{
    cryptonight_hash_r, cryptonight_hash_v0, cryptonight_hash_v1, cryptonight_hash_v2,
};

criterion_group! {
    name = benches;
    // Criterion suggests that higher measurement time is required for these hash functions.
    config = Criterion::default().measurement_time(Duration::from_secs(8));
    targets =
    r_8, r_64, r_512, r_4096, r_65536,
    v0_8, v0_64, v0_512, v0_4096, v0_65536,
    v1_8, v1_64, v1_512, v1_4096, v1_65536,
    v2_8, v2_64, v2_512, v2_4096, v2_65536,
}

criterion_main!(benches);

/// Generate the benchmark functions for the cryptonight hash functions.
macro_rules! impl_hash_benchmark {
    ($(
        // The actual hash function.
        $hash_fn:ident {
            // Inside these braces:
            // - The name of the benchmark function
            // - The input(s) to the hash function for that benchmark function
            $(
                $fn_name:ident => ($($input:expr_2021),* $(,)?)
            ),* $(,)?
        }
    )*) => {
        $(
            $(
                #[named]
                fn $fn_name(c: &mut Criterion) {
                    c.bench_function(function_name!(), |b| {
                        b.iter(|| {
                            drop(
                                black_box(
                                    $hash_fn(
                                        $(black_box($input)),*
                                    )
                                )
                            );
                        });
                    });
                }
            )*
        )*
    };
}

impl_hash_benchmark! {
    cryptonight_hash_r {
        r_8     => (&[3; 8],    500_000),
        r_64    => (&[3; 64],   500_000),
        r_512   => (&[3; 512],  500_000),
        r_4096  => (&[3; 4096], 500_000),
        r_65536 => (&[3; 65536], 500_000),
    }

    cryptonight_hash_v0 {
        v0_8     => (&[3; 8]),
        v0_64    => (&[3; 64]),
        v0_512   => (&[3; 512]),
        v0_4096  => (&[3; 4096]),
        v0_65536 => (&[3; 65536]),
    }

    cryptonight_hash_v1 {
        v1_8     => (&[3; 8]),
        v1_64    => (&[3; 64]),
        v1_512   => (&[3; 512]),
        v1_4096  => (&[3; 4096]),
        v1_65536 => (&[3; 65536]),
    }

    cryptonight_hash_v2 {
        v2_8     => (&[3; 8]),
        v2_64    => (&[3; 64]),
        v2_512   => (&[3; 512]),
        v2_4096  => (&[3; 4096]),
        v2_65536 => (&[3; 65536]),
    }
}
