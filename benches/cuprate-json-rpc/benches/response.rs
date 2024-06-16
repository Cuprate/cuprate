//! `trait Storable` benchmarks.

//---------------------------------------------------------------------------------------------------- Import
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use function_name::named;
use serde_json::{from_str, to_string_pretty};

use json_rpc::{Id, Response};

//---------------------------------------------------------------------------------------------------- Criterion
criterion_group! {
    benches,
    from_str_u8,
    from_str_u64,
    from_str_string_5_len,
    from_str_string_10_len,
    from_str_string_100_len,
    from_str_string_500_len,
    to_string_pretty_u8,
    to_string_pretty_u64,
    to_string_pretty_string_5_len,
    to_string_pretty_string_10_len,
    to_string_pretty_string_100_len,
    to_string_pretty_string_500_len,
}
criterion_main!(benches);

//---------------------------------------------------------------------------------------------------- Deserialization
/// TODO
macro_rules! impl_from_str_benchmark {
    (
        $(
            $fn_name:ident => $request_type:ty => $request_string:literal,
        )*
    ) => {
        $(
            /// TODO
            #[named]
            fn $fn_name(c: &mut Criterion) {
                let request_string = $request_string;

                c.bench_function(function_name!(), |b| {
                    b.iter(|| {
                        black_box(
                            from_str::<Response<$request_type>>(
                                black_box(request_string)
                            )
                        ).unwrap();
                    });
                });
            }
        )*
    };
}

impl_from_str_benchmark! {
    from_str_u8 => u8 => r#"{"jsonrpc":"2.0","id":123,"result":0}"#,
    from_str_u64 => u64 => r#"{"jsonrpc":"2.0","id":123,"result":0}"#,
    from_str_string_5_len => String => r#"{"jsonrpc":"2.0","id":123,"result":"hello"}"#,
    from_str_string_10_len => String => r#"{"jsonrpc":"2.0","id":123,"result":"hellohello"}"#,
    from_str_string_100_len => String => r#"{"jsonrpc":"2.0","id":123,"result":"helloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworld"}"#,
    from_str_string_500_len => String => r#"{"jsonrpc":"2.0","id":123,"result":"helloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworld"}"#,
}

//---------------------------------------------------------------------------------------------------- Deserialization
/// TODO
macro_rules! impl_to_string_pretty_benchmark {
    (
        $(
            $fn_name:ident => $request_constructor:expr,
        )*
    ) => {
        $(
            /// TODO
            #[named]
            fn $fn_name(c: &mut Criterion) {
                let request = $request_constructor;

                c.bench_function(function_name!(), |b| {
                    b.iter(|| {
                        black_box(to_string_pretty(black_box(&request))).unwrap();
                    });
                });
            }
        )*
    };
}

impl_to_string_pretty_benchmark! {
    to_string_pretty_u8 => Response::<u8>::ok(Id::Null, 0),
    to_string_pretty_u64 => Response::<u64>::ok(Id::Null, 0),
    to_string_pretty_string_5_len => Response::ok(Id::Null, String::from("hello")),
    to_string_pretty_string_10_len => Response::ok(Id::Null, String::from("hellohello")),
    to_string_pretty_string_100_len => Response::ok(Id::Null, String::from("helloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworld")),
    to_string_pretty_string_500_len => Response::ok(Id::Null, String::from("helloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworld")),
}
