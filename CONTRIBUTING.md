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
The first 3 steps to CI are formatting, typo, and documentation checking.

Check if your changes are formatted, typo-free, and documented correctly by running:
- `cargo fmt --all --check`
- `typos`
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features`

`typos` can be installed with `cargo` from: https://github.com/crate-ci/typos.

After that, ensure all lints, tests, and builds are successful by running:

- `cargo clippy --workspace --all-features --all-targets -- -D warnings`
- `cargo fmt --all`
- `cargo test --all-features --workspace`
- `cargo build --all-features --all-targets --workspace`

## Coding guidelines

- `// Comment like this.` and not `//like this`
- Use `TODO` instead of `FIXME`
- Avoid `unsafe`
- Sort imports as core, std, third-party, Cuprate crates, current crate.
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines)
- Break the above rules when it makes sense

## Keeping track of issues and PRs
The Cuprate GitHub repository has a lot of issues and PRs to keep track of. Cuprate makes use of generic labels and labels grouped by a prefixes to help with this.

The following section explains the meaning of various labels used.
This section is primarily targeted at maintainers. Most contributors aren't able to set these labels.

| Labels       | Description | Example |
|--------------|-------------|---------|
| [A-]         | The **area** of the project an issue relates to. | `A-database`, `A-RPC`, `A-docs`
| [C-]         | The **category** of an issue. | `C-cleanup`,  `C-enhancement`, `C-optimization`
| [D-]         | Issues for **diagnostics**. | `D-confusing`, `D-verbose`
| [E-]         | The **experience** level necessary to fix an issue. | `E-easy`, `E-hard`
| [I-]         | The **importance** of the issue. | `I-crash`, `I-slow`,  `I-memleak`
| [O-]         |  The **operating system** or platform that the issue is specific to. | `O-windows`, `O-macos`, `O-linux`
| [P-]         | The issue **priority**. These labels can be assigned by anyone that understand the issue and is able to prioritize it, and remove the [I-prioritize] label. | `P-high`, `P-low`
| [regression] | Tracks regressions from a release. | |
| [wontfix]    | Indicates an issue will not be fixed. | |

[A-]: https://github.com/Cuprate/cuprate/labels?q=A
[C-]: https://github.com/Cuprate/cuprate/labels?q=C
[D-]: https://github.com/Cuprate/cuprate/labels?q=D
[E-]: https://github.com/Cuprate/cuprate/labels?q=E
[I-]: https://github.com/Cuprate/cuprate/labels?q=I
[O-]: https://github.com/Cuprate/cuprate/labels?q=O
[P-]: https://github.com/Cuprate/cuprate/labels?q=P
[regression]: https://github.com/Cuprate/cuprate/labels?q=regression
[wontfix]: https://github.com/Cuprate/cuprate/labels?q=wontfix