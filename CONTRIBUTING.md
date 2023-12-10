# Contributing to Cuprate

## Introduction
TODO

## Filing an issue
TODO

## Making a PR
TODO

## Passing CI
TODO

- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all`
- `cargo test`
- `cargo build`

## Coding guidelines
- Add blank lines around all `fn`, `struct`, `enum`, etc
- `// Comment like this.` and not `//like this`
- Use `TODO` instead of `FIXME`
- Avoid `unsafe`
- Add some example code (doc-tests)
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines)
- Break the above rules when it makes sense
