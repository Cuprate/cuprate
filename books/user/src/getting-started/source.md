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

### General distributions
```bash
# Debian/Ubuntu
sudo apt install -y build-essential cmake git

# Arch
sudo pacman -Syu base-devel cmake git

# Fedora
sudo dnf install @development-tools gcc gcc-c++ cmake git
```

### NixOS 
```bash
# Create shell.nix file
nano shell.nix
# and paste the following:
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = [
    pkgs.cargo
    pkgs.cmake
    pkgs.git
    pkgs.lld
    pkgs.patchelf
    pkgs.pkg-config
    pkgs.rustc
    pkgs.stdenv.cc
  ];

  # Deliberately do not set LD_LIBRARY_PATH globally.

  RUSTFLAGS = "-C link-arg=-fuse-ld=lld";

  shellHook = ''
    export CUPRATE_CXX_LIB_DIR="${pkgs.stdenv.cc.cc.lib}/lib"
  '';
}

# then save the file with Ctrl+x
# and finally
nix-shell
```
NOTE: NixOS requires some special instructions. see [Running](https://user.cuprate.org/getting-started/run.html) for details

### Clone the Cuprate repository and build:

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
