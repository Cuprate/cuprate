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
        .flag("-O3")
        .flag("-fexceptions")
        // c/oaes_lib.c: In function ‘oaes_get_seed’:
        // c/oaes_lib.c:515:9: warning: ‘ftime’ is deprecated: Use gettimeofday or clock_gettime instead [-Wdeprecated-declarations]
        //   515 |         ftime (&timer);
        //       |         ^~~~~
        // In file included from c/oaes_lib.c:45:
        // /usr/include/sys/timeb.h:29:12: note: declared here
        //    29 | extern int ftime (struct timeb *__timebuf)
        //       |            ^~~~~
        .flag("-Wno-deprecated-declarations");

    let target = env::var("TARGET").unwrap();
    if target.contains("x86_64") {
        cfg.file("c/CryptonightR_template.S")
            .flag("-maes")
            .flag("-msse2");
    }

    cfg.compile("cryptonight")
}
