extern crate cc;

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
        .file("c/memwipe.c")
        .file("c/slow-hash.c")
        .flag_if_supported("-fexceptions")
        .flag_if_supported("-Wno-deprecated-declarations");

    // Optimization flags are automatically added.
    // https://docs.rs/cc/latest/cc/struct.Build.html#method.opt_level

    cfg.compile("cryptonight");
}
