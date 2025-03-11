# Platform support

Support for different platforms ("targets") are organized into three tiers,
each with a different set of guarantees. Targets are identified by the
[Rust "target triple"](https://doc.rust-lang.org/rustc/platform-support.html)
which is the string used when compiling `cuprated`.

| Attribute           | Tier 1 | Tier 2            | Tier 3 |
|---------------------|--------|-------------------|--------|
| Official builds     | 🟢     | 🟢                | 🔴
| Guaranteed to build | 🟢     | 🟢                | 🟡
| Automated testing   | 🟢     | 🟡 (some targets) | 🔴
| Manual testing      | 🟢     | 🟡 (sometimes)    | 🔴

## Tier 1

Tier 1 targets can be thought of as "guaranteed to work".

| Target                      | Notes  |
|-----------------------------|--------|
| `x86_64-unknown-linux-gnu`  | x64 Linux (glibc 2.36+)
| `aarch64-unknown-linux-gnu` | ARM64 Linux (glibc 2.36+)
| `aarch64-apple-darwin`      | ARM64 macOS (11.0+)

## Tier 2

Tier 2 targets can be thought of as "guaranteed to build".

| Target                      | Notes  |
|-----------------------------|--------|
| `x86_64-pc-windows-msvc`    | x64 Windows (MSVC, Windows Server 2022+)

## Tier 3

Tier 3 targets are those which the Cuprate codebase likely can support,
but which Cuprate does not build or test on a regular basis, so they may or may not work.
Official builds are not available, but may eventually be planned.

| Target                       | Notes  |
|------------------------------|--------|
| `x86_64-unknown-linux-musl`  | x64 Linux (musl 1.2.3)
| `aarch64-unknown-linux-musl` | ARM64 Linux (musl 1.2.3)
| `x86_64-unknown-freebsd` 	   | x64 FreeBSD
| `aarch64-unknown-freebsd`    | ARM64 FreeBSD
| `aarch64-pc-windows-msvc`    | ARM64 Windows (MSVC, Windows Server 2022+)
| `x86_64-apple-darwin`        | x64 macOS
