# Contributing to Cuprate

## Introduction

Thank you for wanting to help out! Cuprate is in the stage where things are likely to change quickly, so it's recommend
you join our [Matrix room](https://matrix.to/#/#cuprate:monero.social).

## Making a PR

Once you have found something you would like to work on by either looking at the open issues or joining Cuprate's [Matrix room](https://matrix.to/#/#cuprate:monero.social)
and asking it's recommended to make your interest on working on that thing known so people don't duplicate work.

When you are at a stage where you would like feedback you can open a draft PR, keep in mind that feedback may take time especially if the change is large.
Once your PR is at the stage where you feel it's ready to go, open it for review.

## Passing CI

To pass CI make sure all these successfully run:

- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all`
- `cargo test`
- `cargo build`

## Coding guidelines

- `// Comment like this.` and not `//like this`
- Use `TODO` instead of `FIXME`
- Avoid `unsafe`
- Sort imports as core, std, third-party, Cuprate crates, current crate.
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines)
- Break the above rules when it makes sense
