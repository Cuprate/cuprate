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
        .flag_if_supported("-fexceptions")
        // c/oaes_lib.c: In function ‘oaes_get_seed’:
        // c/oaes_lib.c:515:9: warning: ‘ftime’ is deprecated: Use gettimeofday or clock_gettime instead [-Wdeprecated-declarations]
        //   515 |         ftime (&timer);
        //       |         ^~~~~
        // In file included from c/oaes_lib.c:45:
        // /usr/include/sys/timeb.h:29:12: note: declared here
        //    29 | extern int ftime (struct timeb *__timebuf)
        //       |            ^~~~~
        // This flag doesn't work on MSVC and breaks CI.
        .flag_if_supported("-Wno-deprecated-declarations");

    // Optimization flags are automatically added.
    // https://docs.rs/cc/latest/cc/struct.Build.html#method.opt_level

    let target = env::var("TARGET").unwrap();
    if target.contains("x86_64") {
        // FIXME: what are the equivalent flags for MSVC?
        cfg.file("c/CryptonightR_template.S")
            .flag_if_supported("-maes")
            .flag_if_supported("-msse2");
    }

    cfg.compile("cryptonight")
}
