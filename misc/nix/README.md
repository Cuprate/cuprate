This directory contains a Nix package definition for building and installing cuprated. The package includes its C++ runtime dependencies, so the installed executable does not require LD_LIBRARY_PATH or a development shell.

# Requirements

* Nix installed on Linux.
* A Git checkout of the Cuprate repository.
* A configured <nixpkgs> search-path entry with a sufficiently recent Rust toolchain.
* Network access to fetch build dependencies.

The commands below use traditional Nix expressions. Flakes and experimental Nix features are not required.
# Building

Run these commands from the repository root:

bash
```
nix-build ./misc/nix \
  --argstr rev "$(git rev-parse HEAD)"
```
After a successful build, Nix creates a result symlink pointing to the package in the Nix store.

Test the executable:

bash
```
./result/bin/cuprated -h
```

This package builds the `cuprated` executable in release mode. The current definition does not run the test suite.

# Version and commit metadata
The package version is read automatically from `binaries/cuprated/Cargo.toml`.

The `rev` argument supplies the full Git commit hash through the `GITHUB_SHA` build environment variable. Cuprate uses this to embed its commit metadata without requiring `.git` in the build sandbox.

Always supply the revision for the checkout being packaged:

Uncommitted source changes are included in the local build, subject to the source filter, but the embedded hash still identifies the base commit. Use a clean, committed checkout when producing release or shared-cache builds.

# Installing
To install the package in your user profile:

bash
```
nix-env -i -f ./misc/nix \
  --argstr rev "$(git rev-parse HEAD)"
```
You can then run it from any directory with `cuprated -h`

No `nix-shell` or runtime environment variable is needed.

While the package remains installed in your profile, Nix retains the package and its referenced runtime dependencies during garbage collection. The `result` symlink is not needed after installation.

To install an updated checkout, run the installation command again with its current commit hash.

Installation does not configure or start a systemd service.

To uninstall: run `nix-env -e cuprate`

# Updating dependency hashes
The `cargoHash` attribute in `default.nix` verifies the vendored Cargo dependencies, including Git dependencies.

If a dependency update changes the vendored dependency output:

1. Temporarily replace the existing hash with:

Nix
```
cargoHash = pkgs.lib.fakeHash;
```

2. Run the build:

bash
```
nix-build ./misc/nix \
  --argstr rev "$(git rev-parse HEAD)"
```

3. The build should fail with a hash mismatch. Copy the complete `got hash` into the definition:

Nix
```
cargoHash = "sha256-REPLACE-WITH-THE-REPORTED-HASH";
```

4. Rebuild and commit the updated hash alongside the dependency changes.

Keep the hash quoted and preserve any trailing `=` characters.

Ordinary source changes do not generally require a new dependency hash. Do not replace the hash merely because compilation failed; inspect the error first.

# Selecting nixpkgs

By default, the package imports `<nixpkgs>`. Its Rust toolchain, CMake, compiler, and runtime libraries come from that package set.

You can supply a different nixpkgs checkout explicitly:

bash
```
nix-build ./misc/nix \
  --arg pkgs 'import /path/to/pinned/nixpkgs {}' \
  --argstr rev "$(git rev-parse HEAD)"
```
For reproducible builds and reliable binary-cache hits, use a pinned nixpkgs revision rather than a moving channel. CI and users should use the same package inputs.

# Build reuse and binary caching

Nix reuses an existing build when the same derivation is already available in the local store or a configured binary cache.

A custom Cuprate build is not automatically available from the official NixOS binary cache. Without a matching cached build, Nix compiles it locally.

A project cache can distribute builds produced by CI. For example, after configuring and authenticating [Cachix:](https://www.cachix.org/)

bash
```
nix-build ./misc/nix \
  --argstr rev "$(git rev-parse HEAD)"

cachix push YOUR-CACHE-NAME ./result
```

Consumers must configure the cache and use matching package inputs to download that build instead of compiling it.

Cache upload credentials must remain outside the repository. CI should upload only from trusted builds and must not expose upload credentials to untrusted pull requests.

# Troubleshooting
## Nix looks for `flake.nix`
Use nix-build with a hyphen:

bash
```
nix-build ./misc/nix \
  --argstr rev "$(git rev-parse HEAD)"
```
Plain `nix build` defaults to flake-based operation. This package does not require a flake.

## A build fails
Request a more detailed evaluation trace:

bash
```
nix-build ./misc/nix \
  --argstr rev "$(git rev-parse HEAD)" \
  --show-trace
```
For compilation failures, inspect the build output. Nix also prints the derivation path, whose complete log can be read with:

bash
```
nix-store -l /nix/store/REPLACE-WITH-THE-DERIVATION.drv
```
