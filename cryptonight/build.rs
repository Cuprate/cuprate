extern crate cc;

use cc::Build;

fn main() {
    Build::new()
        .include("c")
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
        .flag("-maes")
        .flag("-Ofast")
        .flag("-fexceptions")
        .compile("cryptonight")
}
