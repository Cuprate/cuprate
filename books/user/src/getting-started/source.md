# Building from source
To build `cuprated` from source you will need:

- `git`
- Up-to-date Rust toolchain
- Compiler toolchain
- Certain system dependencies

To install Rust, follow [these instructions](https://www.rust-lang.org/learn/get-started) or run:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

<!-- TODO: Windows build instruction -->

## Linux
Install the required system dependencies:

```bash
# Debian/Ubuntu
sudo apt install -y build-essentials cmake git

# Arch
sudo pacman -Syu base-devel cmake git

# Fedora
sudo dnf install @development-tools gcc gcc-c++ cmake git
```

Clone the Cuprate repository and build:

```bash
git clone https://github.com/Cuprate/cuprate
cd cuprate/
cargo build --release --package cuprated
```

The built `cuprated` binary should be located at `target/release/cuprated`.

## macOS
Install [Homebrew](https://brew.sh):

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Install the required system dependencies:
```bash
brew install cmake
```

Clone the Cuprate repository and build:

```bash
git clone https://github.com/Cuprate/cuprate
cd cuprate/
cargo build --release --package cuprated
```

The built `cuprated` binary should be located at `target/release/cuprated`.
