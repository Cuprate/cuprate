#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"

export LC_ALL=C
export TZ=UTC

# Hold all the "no network after distsrc creation" invariants up front so a
# later `cargo metadata` or anything else can't accidentally hit the network.
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true

dist_src="${GUIX_DIST_SRC:-}"
if [[ -z "$dist_src" || ! -f "$dist_src" ]]; then
  echo "GUIX_DIST_SRC must point to a source archive produced by contrib/guix/mk-distsrc" >&2
  exit 1
fi

# Use a per-run build root so two concurrent invocations in the same checkout
# don't trample each other. Inside `guix shell --container --pure`, `$TMPDIR`
# is private to the container, so collisions across simultaneous outer
# invocations are also avoided.
build_root="$(mktemp -d "${TMPDIR:-/tmp}/guix-build-src.XXXXXX")"
src_dir="$build_root/src"
aside_dir="$build_root/distsrc-aside"
cleanup_build_root() {
  # On success, drop the tree. On failure keep it so the user can inspect
  # state - this is hugely valuable when distsrc verification or CMake
  # configure fails inside a sealed container.
  local code=$?
  # If the distsrc-equivalence check exited mid-way (between move-aside
  # and the explicit restore), put the three legitimately-added paths
  # back so the preserved tree reflects the original distsrc contents.
  # Idempotent: no-op if aside_dir was already cleaned up on success.
  if [[ -d "$aside_dir" ]]; then
    [[ -e "$aside_dir/vendor"             ]] && mv "$aside_dir/vendor"             "$src_dir/vendor"             2>/dev/null || true
    [[ -e "$aside_dir/cargo"              ]] && mv "$aside_dir/cargo"              "$src_dir/.cargo"             2>/dev/null || true
    [[ -e "$aside_dir/distsrc-meta.json"  ]] && mv "$aside_dir/distsrc-meta.json"  "$src_dir/.cuprate-distsrc.json" 2>/dev/null || true
    rmdir "$aside_dir" 2>/dev/null || true
  fi
  if [[ $code -ne 0 ]]; then
    echo "build failed; preserving build root for inspection: $build_root" >&2
  else
    rm -rf "$build_root"
  fi
}
trap cleanup_build_root EXIT

export CARGO_HOME="$build_root/cargo-home"
mkdir -p "$src_dir" "$CARGO_HOME"

# See mk-distsrc: guix shell --container runs as a mapped non-root user that
# cannot honor the archive's stored uid/gid; pass --no-same-owner so tar
# accepts the unprivileged extraction.
tar -xf "$dist_src" --no-same-owner --no-same-permissions -C "$src_dir"

# Parse the manifest we just extracted. cargo can pick up its own
# vendor config now that CARGO_HOME and CARGO_NET_OFFLINE are set.
version="$({ cd "$src_dir" && cargo metadata --locked --format-version=1 --no-deps | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="cuprated"))'; })"
: "${version:?unable to parse version}"

SOURCE_DATE_EPOCH="$(python3 -c 'import json; print(json.load(open("'"$src_dir"'/.cuprate-distsrc.json"))["source_date_epoch"])')"
git_commit="$(python3 -c 'import json; print(json.load(open("'"$src_dir"'/.cuprate-distsrc.json"))["git_commit"])')"
distsrc_sha256="$(sha256sum "$dist_src" | awk '{print $1}')"
outer_commit="$(git -C "$repo_root" rev-parse HEAD)"
if [[ "$outer_commit" != "$git_commit" && "${GUIX_ALLOW_COMMIT_MISMATCH:-0}" != "1" ]]; then
  echo "outer checkout $outer_commit differs from distsrc commit $git_commit" >&2
  exit 1
fi
export SOURCE_DATE_EPOCH

# cuprate-constants/build.rs hardcodes the embedded git commit by running
# `git show -s --format=%H` in CARGO_MANIFEST_DIR (relying on git walking up
# to find a .git/). The distsrc tarball legitimately contains no .git/, so
# `git show` finds nothing and the build script's assert(commit.len() == 40)
# trips. constants/build.rs also honors a GITHUB_SHA env var as an override -
# set it from the distsrc's authoritative git_commit so the embedded commit
# is bound to the distsrc, not to whatever outer checkout the build happens
# to run inside.
export GITHUB_SHA="$git_commit"

# Distsrc content equivalence: verify the extracted source tree matches
# `git archive $git_commit` from the outer checkout. The three paths
# `mk-distsrc` legitimately adds (vendor/, .cargo/, .cuprate-distsrc.json)
# are moved aside before the diff and restored afterwards, so any nested
# directory or file that *happens* to share one of those basenames is
# still compared. This catches a tampered distsrc that lies about its
# git_commit but contains modified source files. The git_commit equality
# check above is metadata-only and not sufficient on its own.
#
# The cleanup_build_root EXIT trap installed near the top of this script
# also restores the aside-d paths on any failure that lands here mid-way,
# so the preserved $build_root reflects the as-extracted distsrc.
verify_dir="$build_root/git-baseline"
mkdir -p "$verify_dir"
git -C "$repo_root" archive --format=tar "$git_commit" | \
  tar -xf - --no-same-owner --no-same-permissions -C "$verify_dir"
mkdir -p "$aside_dir"
mv "$src_dir/vendor"                 "$aside_dir/vendor"
mv "$src_dir/.cargo"                 "$aside_dir/cargo"
mv "$src_dir/.cuprate-distsrc.json"  "$aside_dir/distsrc-meta.json"
if ! diff -rq "$verify_dir" "$src_dir" > "$build_root/distsrc-diff.log" 2>&1; then
  echo "ERROR: distsrc contents diverge from git commit $git_commit" >&2
  echo "see diff log:" >&2
  cat "$build_root/distsrc-diff.log" >&2
  exit 1
fi
mv "$aside_dir/vendor"             "$src_dir/vendor"
mv "$aside_dir/cargo"               "$src_dir/.cargo"
mv "$aside_dir/distsrc-meta.json"   "$src_dir/.cuprate-distsrc.json"
rmdir "$aside_dir"
rm -rf "$verify_dir"

cd "$src_dir"

# RandomX builds with CMake's ARCH default ("default"), which uses
# compiler-capability-gated -maes -mssse3 -mavx2 - all compiler-deterministic,
# none host-CPU-specific. The bundled randomx-rs build.rs passes
# .define("DARCH", "native"), but cmake reads ARCH, not DARCH, so that line
# is a silent no-op (a years-old typo that fortuitously keeps the build
# portable). See contrib/guix/README.md > "RandomX" for the longer story.
export RUSTFLAGS="--remap-path-prefix=$src_dir=/cuprate -C codegen-units=1"
export CFLAGS="-ffile-prefix-map=$src_dir=/cuprate"

# Workaround: Guix gcc-15.2 libstdc++ ships with both _GLIBCXX_HAVE_FENV_H
# and _GLIBCXX_USE_C99_FENV undefined in bits/c++config.h, so <cfenv> never
# pulls in glibc's <fenv.h> and `fesetround` is absent in the global
# namespace - even though the underlying glibc 2.41 obviously has it. Define
# both macros so any C++ caller of <cfenv> (RandomX, plus future C++ deps)
# compiles cleanly.
#
# This block is REMOVABLE once Guix's libstdc++ packaging is fixed upstream.
# To check: build with GUIX_SKIP_FENV_WORKAROUND=1 and see if RandomX still
# compiles. If yes, this whole `if` block can be deleted.
fenv_workaround=""
if [[ "${GUIX_SKIP_FENV_WORKAROUND:-0}" != "1" ]]; then
  fenv_workaround=" -D_GLIBCXX_HAVE_FENV_H=1 -D_GLIBCXX_USE_C99_FENV=1"
fi
export CXXFLAGS="-ffile-prefix-map=$src_dir=/cuprate${fenv_workaround}"
# Guix's gcc-toolchain profile only provides `gcc`/`g++`, not the legacy `cc`
# alias; cc-rs (used by -sys crates such as libsqlite3-sys, openssl-sys,
# randomx-rs, ring, etc.) defaults to `cc` and fails with
#   ToolNotFound: failed to find tool "cc": No such file or directory
# Pointing CC/CXX/AR/AS at the actual binaries fixes every -sys crate.
export CC=gcc
export CXX=g++
export AR=ar
export AS=as
export LD=ld
export RANLIB=ranlib
export STRIP=strip
# Force openssl-sys to use the system openssl provided by the manifest via
# pkg-config rather than building it from source via openssl-src (which would
# also need `make`, but is wasteful when we already ship openssl).
export OPENSSL_NO_VENDOR=1
# Some -sys crates use $GUIX_ENVIRONMENT to find headers/libs when pkg-config
# is not available; provide explicit hints.
if [[ -n "${GUIX_ENVIRONMENT:-}" ]]; then
  export OPENSSL_DIR="$GUIX_ENVIRONMENT"
  export PKG_CONFIG_PATH="$GUIX_ENVIRONMENT/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
fi

rust_target="${GUIX_RUST_TARGET:-x86_64-unknown-linux-gnu}"

out_dir="${GUIX_OUT_DIR:-$repo_root/contrib/guix/out}"
mkdir -p "$out_dir"

# Stream verbose-makefile + verbose-build output to a file so smoke tests
# can mechanically grep for -march=native / -mcpu=native / target-cpu=native
# regressions (RandomX or any future cc-rs crate could re-introduce them).
build_log="$out_dir/build-${rust_target}.log"
{
  cargo build --frozen --release --package cuprated --target "$rust_target" --verbose
} 2>&1 | tee "$build_log"

bash "$repo_root/contrib/guix/libexec/package.sh" "$version" "$rust_target" "$SOURCE_DATE_EPOCH" "$out_dir" "$src_dir"

binary="$src_dir/target/${rust_target}/release/cuprated"
# ldd output is host/loader-dependent and varies by system; keep it as a
# diagnostic file only - it MUST NOT be included in SHA256SUMS or
# attestation. guix-checksums excludes it explicitly.
if command -v ldd >/dev/null 2>&1; then
  ldd "$binary" > "$out_dir/ldd-${rust_target}.diag.txt" 2>&1 || true
fi

rustc --version --verbose > "$out_dir/rustc-version.txt"
cargo --version --verbose > "$out_dir/cargo-version.txt"

cat > "$out_dir/build-metadata.json" <<META
{
  "package": "cuprated",
  "version": "$version",
  "guix_system": "${GUIX_BUILD_SYSTEM:-x86_64-linux}",
  "rust_target": "$rust_target",
  "source_date_epoch": $SOURCE_DATE_EPOCH,
  "git_commit": "$git_commit",
  "randomx_arch": "default (CMake default; build is host-CPU-independent)",
  "distsrc": "$(basename "$dist_src")",
  "distsrc_sha256": "$distsrc_sha256"
}
META
