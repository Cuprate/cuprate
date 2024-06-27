## Contributing to Cuprate
Thank you for wanting to help out!

Cuprate is in the stage where things are likely to change quickly, so it's recommended
you ask questions in our public [Matrix room](https://matrix.to/#/#cuprate:monero.social).

- [1. Submitting an issue](#1-submitting-an-issue)
	- [1.1 Discussion](#11-discussion)
	- [1.2 Proposal](#12-proposal)
	- [1.3 Tracking issue](#13-tracking-issue)
- [2. Submitting a pull request](#2-submitting-a-pull-request)
	- [2.1 Rust toolchain](#21-rust-toolchain)
	- [2.2 Draft PR](#22-draft-pr)
	- [2.3 Passing CI](#23-passing-ci)
	- [2.4 Ready for review](#24-ready-for-review)
- [3. Keeping track of issues and PRs](#3-keeping-track-of-issues-and-prs)
	- [3.1 Labels](#31-labels)
	- [3.2 Tracking issues](#32-tracking-issues)
- [4. Coding guidelines](#4-coding-guidelines)
	- [4.1 General guidelines](#41-general-guidelines)
	- [4.2 Crate names](#42-crate-names)
	- [4.3 Pull request title and description](#43-pull-request-title-and-description)
- [5. Documentation](#5-documentation)
- [6. Books](#6-books)
	- [6.1 Architecture book](#61-architecture-book)
	- [6.2 Protocol book](#62-protocol-book)
	- [6.3 User book](#63-user-book)

## 1. Submitting an issue
Before starting work, consider opening an issue for discussion.

If you have a plan already, you can jump straight into [submitting a pull request](#2-submitting-a-pull-request).

Otherwise, see below for issue types and what they're used for.

### 1.1 Discussion
These are for general discussion on topics that have questions that aren't fully answered yet.

If you would like to discuss a topic and get some feedback, consider [opening a discussion](https://github.com/Cuprate/cuprate/issues/new/choose).

Examples:
- https://github.com/Cuprate/cuprate/issues/40
- https://github.com/Cuprate/cuprate/issues/53
- https://github.com/Cuprate/cuprate/issues/163

### 1.2 Proposal
These are formal issues that specify changes that are _almost_ ready for implementation.

These should answer some basic questions:
- **What** is this proposal for?
- **Why** is this proposal needed?
- **Where** will this proposal make changes to?
- **How** will this proposal be implemented?

If you have a close to fully fleshed out idea, consider [opening a proposal](https://github.com/Cuprate/cuprate/issues/new/choose).

Opening a PR and writing the proposal in the PR description is also viable.

Examples:
- https://github.com/Cuprate/cuprate/pull/146
- https://github.com/Cuprate/cuprate/issues/106
- https://github.com/Cuprate/cuprate/issues/153
- https://github.com/Cuprate/cuprate/issues/181

### 1.3 Tracking issue
These are meta-issues that track an in-progress implementation.

See [`Tracking issues`](#32-tracking-issues) for more info.

## 2. Submitting a pull request
Once you have found something you would like to work on after:
- Discussing an idea on an [issue](#1-submitting-an-issue)
- Looking at the [open issues](https://github.com/Cuprate/cuprate/issues)
- Looking at issues with the [`A-help-wanted`](https://github.com/Cuprate/cuprate/issues?q=is%3Aissue+is%3Aopen+label%3AE-help-wanted) label
- Joining Cuprate's [Matrix room](https://matrix.to/#/#cuprate:monero.social) and asking

it is recommended to make your interest on working on that thing known so people don't duplicate work.

Before starting, consider reading/using Cuprate's:
- [`Documentation`](#5-documentation)
- [`Books`](#6-books)

These may answer some questions you have, or may confirm an issue you would like to fix.

_Note: Cuprate is currently a work-in-progress; documentation will be changing/unfinished._

### 2.1 Rust toolchain
Cuprate is written in [Rust](https://rust-lang.org).

If you are editing code, you will need Rust's toolchain and package manager,
[`cargo`](https://doc.rust-lang.org/cargo/index.html), to develop and submit PRs effectively.

Get started with Rust here: <https://www.rust-lang.org/learn/get-started>.

### 2.2 Draft PR
Consider opening a draft PR until you have passed all CI.

This is also the stage where you can ask for feedback from others. Keep in mind that feedback may take time especially if the change is large.

### 2.3 Passing CI
Each commit pushed in a PR will trigger our [lovely, yet pedantic CI](https://github.com/Cuprate/cuprate/blob/main/.github/workflows/ci.yml).

It currently:
- Checks code formatting
- Checks documentation
- Looks for typos
- Runs [`clippy`](https://github.com/rust-lang/rust-clippy) (and fails on warnings)
- Runs all tests
- Builds all targets
- Automatically adds appropriate [labels](#31-labels) to your PR

Before pushing your code, please run the following at the root of the repository:

| Command           | Does what |
|-------------------|-----------|
| `cargo fmt --all` | Formats code
| `typos -w`        | Fixes typos

`typos` can be installed with `cargo` from: https://github.com/crate-ci/typos.

After that, ensure all other CI passes by running:

| Command                                                                                    | Does what                                                             |
|--------------------------------------------------------------------------------------------|-----------------------------------------------------------------------|
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features`                          | Checks documentation is OK                                            |
| `cargo clippy --workspace --all-features --all-targets -- -D warnings`                     | Checks clippy lints are satisfied                                     |
| `cargo test --all-features --workspace`                                                    | Runs all tests                                                        |
| `cargo build --all-features --all-targets --workspace`                                     | Builds all code                                                       |
| `cargo hack --workspace --exclude cuprate-database check --feature-powerset --no-dev-deps` | Uses cargo hack to check our crates build with different features set |

**Note: in order for some tests to work, you will need to place a [`monerod`](https://www.getmonero.org/downloads/) binary at the root of the repository.**

### 2.4 Ready for review
Once your PR has passed all CI and is ready to go, open it for review. Others will leave their thoughts and may ask for changes to be made.

Finally, if everything looks good, we will merge your code! Thank you for contributing!

## 3. Keeping track of issues and PRs
The Cuprate GitHub repository has a lot of issues and PRs to keep track of.

This section documents tools used to help with this.

### 3.1 Labels
Cuprate makes use of labels grouped by prefixes.

Some labels will be [automatically added/removed](https://github.com/Cuprate/cuprate/tree/main/.github/labeler.yml) if certain file paths have been changed in a PR.

The following section explains the meaning of various labels used.
This section is primarily targeted at maintainers. Most contributors aren't able to set these labels.

| Prefix       | Description | Example |
|--------------|-------------|---------|
| [A-]         | The **area** of the project an issue relates to. | `A-storage`, `A-rpc`, `A-docs`
| [C-]         | The **category** of an issue. | `C-cleanup`,  `C-optimization`
| [D-]         | Issues for **diagnostics**. | `D-confusing`, `D-verbose`
| [E-]         | The **experience** level necessary to fix an issue. | `E-easy`, `E-hard`
| [I-]         | The **importance** of the issue. | `I-crash`, `I-memory`
| [O-]         | The **operating system** or platform that the issue is specific to. | `O-windows`, `O-macos`, `O-linux`
| [P-]         | The issue **priority**. These labels can be assigned by anyone that understand the issue and is able to prioritize it, and remove the [I-prioritize] label. | `P-high`, `P-low`

[A-]: https://github.com/Cuprate/cuprate/labels?q=A
[C-]: https://github.com/Cuprate/cuprate/labels?q=C
[D-]: https://github.com/Cuprate/cuprate/labels?q=D
[E-]: https://github.com/Cuprate/cuprate/labels?q=E
[I-]: https://github.com/Cuprate/cuprate/labels?q=I
[O-]: https://github.com/Cuprate/cuprate/labels?q=O
[P-]: https://github.com/Cuprate/cuprate/labels?q=P

### 3.2 Tracking issues
If you are working on a larger effort, consider opening a [tracking issue](https://github.com/Cuprate/cuprate/issues/new/choose)!

The main purpose of these are to track efforts that may contain multiple PRs and/or are generally spread out. These don't usually contain the "why", but if they do, they are brief. These contain no implementation details or the how, as those are for the issues/PRs that are being tracked.

Examples:
- https://github.com/Cuprate/cuprate/issues/187
- https://github.com/Cuprate/cuprate/issues/183

## 4. Coding guidelines
These are some rules that are not mandated by any automation, but contributors generally follow.

### 4.1 General guidelines
General guidelines you should keep these in mind when submitting code:

- Separate and sort imports as `core`, `std`, `third-party`, Cuprate crates, current crate
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines)
- `// Comment like this.` and not `//like this`
- Use `TODO` instead of `FIXME`
- Avoid `unsafe`

And the most important rule:
- Break any and all of the above rules when it makes sense

### 4.2 Crate names
All of Cuprate's crates (libraries) are prefixed with `cuprate-`. All directories containing crates however, are not.

For example:

| Crate Directory    | Crate Name         |
|--------------------|--------------------|
| `storage/database` | `cuprate-database` |
| `net/levin`        | `cuprate-levin`    |
| `net/wire`         | `cuprate-wire`     |

### 4.3 Pull request title and description
In general, pull request titles should follow this syntax:
```
<AREA>: <SHORT_DESCRIPTION>
```

For example:
```
books: fix typo
```

The description of pull requests should generally follow the template laid out in [`.github/pull_request_template.md`](.github/pull_request_template.md).

If your pull request is long and/or has sections that need clarifying, consider leaving a review on your own PR with comments explaining the changes.

## 5. Documentation
Cuprate's crates (libraries) have inline documentation.

These can be built and viewed using the `cargo` tool. For example, to build and view a specific crate's documentation, run the following command at the repository's root:
```bash
cargo doc --open --package $CRATE
```
`$CRATE` can be any package listed in the [root `Cargo.toml`](https://github.com/Cuprate/cuprate/tree/main/Cargo.toml)'s workspace members list, for example, `cuprate-blockchain`.

You can also build all documentation at once:
```bash
cargo doc
```
and view by using a web-browser to open the `index.html` file within the build directory: `target/doc/$CRATE/index.html`, for example, `target/doc/cuprate_blockchain/index.html`.

## 6. Books
Cuprate has various documentation books whose source files live in [`books/`](https://github.com/Cuprate/cuprate/tree/main/books).

Please contribute if you found a mistake! The files are mostly [markdown](https://wikipedia.org/wiki/Markdown) files and can be easily edited. See the `books/` directory for more information.

These books are also good resources to understand how Cuprate and Monero work.

### 6.1 Architecture book
This book documents Cuprate's architecture and implementation.

- <https://architecture.cuprate.org>
- <https://github.com/Cuprate/architecture-book>
- <https://github.com/Cuprate/cuprate/tree/main/books/architecture>

### 6.2 Protocol book
This book documents the Monero protocol.

- <https://monero-book.cuprate.org>
- <https://github.com/Cuprate/monero-book>
- <https://github.com/Cuprate/cuprate/tree/main/books/protocol>

### 6.3 User book
This book is a user-guide for using Cuprate.

- <https://user.cuprate.org>
- <https://github.com/Cuprate/user-book>
- <https://github.com/Cuprate/cuprate/tree/main/books/user>
