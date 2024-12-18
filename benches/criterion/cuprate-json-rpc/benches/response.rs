//! Benchmarks for [`Response`].
#![allow(unused_attributes, unused_crate_dependencies)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use function_name::named;
use serde_json::{from_str, to_string_pretty};

use cuprate_json_rpc::{Id, Response};

// `serde` benchmarks on `Response`.
//
// These are benchmarked as `Response` has a custom serde implementation.
criterion_group! {
    name = serde;
    config = Criterion::default();
    targets =
    response_from_str_u8,
    response_from_str_u64,
    response_from_str_string_5_len,
    response_from_str_string_10_len,
    response_from_str_string_100_len,
    response_from_str_string_500_len,
    response_to_string_pretty_u8,
    response_to_string_pretty_u64,
    response_to_string_pretty_string_5_len,
    response_to_string_pretty_string_10_len,
    response_to_string_pretty_string_100_len,
    response_to_string_pretty_string_500_len,
    response_from_str_bad_field_1,
    response_from_str_bad_field_5,
    response_from_str_bad_field_10,
    response_from_str_bad_field_100,
    response_from_str_missing_field,
}
criterion_main!(serde);

/// Generate `from_str` deserialization benchmark functions for [`Response`].
macro_rules! impl_from_str_benchmark {
    (
        $(
            $fn_name:ident => $request_type:ty => $request_string:literal,
        )*
    ) => {
        $(
            #[named]
            fn $fn_name(c: &mut Criterion) {
                let request_string = $request_string;

                c.bench_function(function_name!(), |b| {
                    b.iter(|| {
                        let _r = from_str::<Response<$request_type>>(
                            black_box(request_string)
                        );
                    });
                });
            }
        )*
    };
}

impl_from_str_benchmark! {
    response_from_str_u8 => u8 => r#"{"jsonrpc":"2.0","id":123,"result":0}"#,
    response_from_str_u64 => u64 => r#"{"jsonrpc":"2.0","id":123,"result":0}"#,
    response_from_str_string_5_len => String => r#"{"jsonrpc":"2.0","id":123,"result":"hello"}"#,
    response_from_str_string_10_len => String => r#"{"jsonrpc":"2.0","id":123,"result":"hellohello"}"#,
    response_from_str_string_100_len => String => r#"{"jsonrpc":"2.0","id":123,"result":"helloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworld"}"#,
    response_from_str_string_500_len => String => r#"{"jsonrpc":"2.0","id":123,"result":"helloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworld"}"#,

    // The custom serde currently looks at all fields.
    // These are for testing the performance if the serde
    // has to parse through a bunch of unrelated fields.
    response_from_str_bad_field_1 => u8 => r#"{"bad_field":0,"jsonrpc":"2.0","id":123,"result":0}"#,
    response_from_str_bad_field_5 => u8 => r#"{"bad_field_1":0,"bad_field_2":0,"bad_field_3":0,"bad_field_4":0,"bad_field_5":0,"jsonrpc":"2.0","id":123,"result":0}"#,
    response_from_str_bad_field_10 => u8 => r#"{"bad_field_1":0,"bad_field_2":0,"bad_field_3":0,"bad_field_4":0,"bad_field_5":0,"bad_field_6":0,"bad_field_7":0,"bad_field_8":0,"bad_field_9":0,"bad_field_10":0,"jsonrpc":"2.0","id":123,"result":0}"#,
    response_from_str_bad_field_100 => u8 => r#"{"1":0,"2":0,"3":0,"4":0,"5":0,"6":0,"7":0,"8":0,"9":0,"10":0,"11":0,"12":0,"13":0,"14":0,"15":0,"16":0,"17":0,"18":0,"19":0,"20":0,"21":0,"22":0,"23":0,"24":0,"25":0,"26":0,"27":0,"28":0,"29":0,"30":0,"31":0,"32":0,"33":0,"34":0,"35":0,"36":0,"37":0,"38":0,"39":0,"40":0,"41":0,"42":0,"43":0,"44":0,"45":0,"46":0,"47":0,"48":0,"49":0,"50":0,"51":0,"52":0,"53":0,"54":0,"55":0,"56":0,"57":0,"58":0,"59":0,"60":0,"61":0,"62":0,"63":0,"64":0,"65":0,"66":0,"67":0,"68":0,"69":0,"70":0,"71":0,"72":0,"73":0,"74":0,"75":0,"76":0,"77":0,"78":0,"79":0,"80":0,"81":0,"82":0,"83":0,"84":0,"85":0,"86":0,"87":0,"88":0,"89":0,"90":0,"91":0,"92":0,"93":0,"94":0,"95":0,"96":0,"97":0,"98":0,"99":0,"100":0,"jsonrpc":"2.0","id":123,"result":0}"#,

    // These are missing the `jsonrpc` field.
    response_from_str_missing_field => u8 => r#"{"id":123,"result":0}"#,
}

/// Generate `to_string_pretty` serialization benchmark functions for [`Response`].
macro_rules! impl_to_string_pretty_benchmark {
    (
        $(
            $fn_name:ident => $request_constructor:expr_2021,
        )*
    ) => {
        $(
            #[named]
            fn $fn_name(c: &mut Criterion) {
                let request = $request_constructor;

                c.bench_function(function_name!(), |b| {
                    b.iter(|| {
                        let _s = to_string_pretty(black_box(&request)).unwrap();
                    });
                });
            }
        )*
    };
}

impl_to_string_pretty_benchmark! {
    response_to_string_pretty_u8 => Response::<u8>::ok(Id::Null, 0),
    response_to_string_pretty_u64 => Response::<u64>::ok(Id::Null, 0),
    response_to_string_pretty_string_5_len => Response::ok(Id::Null, String::from("hello")),
    response_to_string_pretty_string_10_len => Response::ok(Id::Null, String::from("hellohello")),
    response_to_string_pretty_string_100_len => Response::ok(Id::Null, String::from("helloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworld")),
    response_to_string_pretty_string_500_len => Response::ok(Id::Null, String::from("helloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworldhelloworld")),
}
