extern crate cc;

use std::env;

use cc::Build;

fn main() {
    let mut cfg = Build::new();
    cfg.include("c")
        .file("c/aesb.c")
        .file("c/blake256.c")
        .file("c/groestl.c")
        .file("c/hash-extra-blake.c")
        .file("c/hash-extra-groestl.c")
        .file("c/hash-extra-jh.c")
        .file("c/hash-extra-skein.c")
        .file("c/hash.c")
        .file("c/jh.c")
        .file("c/keccak.c")
        .file("c/oaes_lib.c")
        .file("c/skein.c")
        .file("c/slow-hash.c")
        .file("c/CryptonightR_JIT.c")
        .file("c/CryptonightR_template.S")
        .flag("-O3")
        .flag("-fexceptions");

    let target = env::var("TARGET").unwrap();
    if target.contains("x86_64") {
        cfg.flag("-maes").flag("-msse2");
    }

    cfg.compile("cryptonight")
}
