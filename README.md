<div align="center">
	<img src="misc/logo/wordmark/CuprateWordmark.svg" width="50%"/>

An alternative Monero node implementation.

_(work-in-progress)_

[![Matrix](https://img.shields.io/badge/Matrix-Cuprate-white?logo=matrix&labelColor=grey&logoColor=white)](https://matrix.to/#/#cuprate:monero.social) [![CI](https://github.com/Cuprate/cuprate/actions/workflows/ci.yml/badge.svg)](https://github.com/Cuprate/cuprate/actions/workflows/ci.yml)

</div>

## Contents
- [About](#about)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [Security](#security)
- [License](#license)

<!--
TODO: add these sections someday.

- [Status](#status) // when we're near v1.0.0
- [Getting help](#getting-help) // issue tracker, user book, matrix channels, etc
- [Build](#build)
	- [Windows](#windows)
	- [macOS](#macOS)
	- [Linux](#Linux)

-->

## About
Cuprate is an alternative [Monero](https://getmonero.org) node implementation, written in [Rust](http://rust-lang.org).

The project is currently a work-in-progress.

## Documentation
_Note that Cuprate is currently a work-in-progress; documentation will be changing/unfinished._

Cuprate maintains various documentation books:
- [Cuprate's architecture book](https://github.com/Cuprate/architecture-book)
- [Cuprate's protocol book](https://github.com/Cuprate/monero-book)

For crate documentation, see the `cargo doc`s of the crates inside the [workspace](Cargo.toml), and the `README.md` inside the crate's directory if applicable, for example: [`storage/cuprate-blockchain/README.md`](storage/cuprate-blockchain/README.md).

## Contributing
See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Security
Cuprate has a responsible vulnerability disclosure policy, see [`SECURITY.md`](SECURITY.md).

## License
Cuprate's components are licensed under either MIT or AGPL-3.0, see [`LICENSE`](LICENSE) for more details.