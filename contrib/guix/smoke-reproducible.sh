#!/usr/bin/env bash
set -euo pipefail

# End-to-end reproducibility self-check.
#
# Builds cuprated twice in two independent checkouts (different paths,
# different temp dirs) and asserts that ALL determinism-sensitive outputs
# match - not just the final tarball. Specifically:
#
#   - cuprate-<v>-<c>-src.tar.gz   (deterministic source archive)
#   - cuprated-<v>-<target>.tar.gz (release artifact)
#   - build-metadata.json          (records SOURCE_DATE_EPOCH, distsrc hash)
#   - rustc-version.txt            (proves manifest pinned the same rustc)
#   - cargo-version.txt            (proves manifest pinned the same cargo)
#   - guix-describe.json           (proves time-machine pinned the same
#                                   channel instance)
#
# Also greps the build log for `-march=native`, `-mcpu=native`, and
# `target-cpu=native` and FAILS if any of them appear - protection against a
# future cc-rs / rustc / CMake change quietly re-introducing host-CPU codegen.

repo_root="$(git rev-parse --show-toplevel)"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/cuprate-smoke.XXXXXX")"

# Preserve the working tree on failure so the user can dig through logs and
# intermediate artifacts. On success, clean up.
on_exit() {
  local code=$?
  if [[ $code -ne 0 ]]; then
    echo "smoke FAILED; preserving working dir for inspection: $tmp" >&2
  else
    rm -rf "$tmp"
  fi
}
trap on_exit EXIT

run_once() {
  local src="$1"
  local commit
  commit="$(git -C "$repo_root" rev-parse HEAD)"

  git clone --quiet --no-local "$repo_root" "$src" >&2
  git -C "$src" checkout --quiet --detach "$commit" >&2

  (
    cd "$src"
    local distsrc artifact
    distsrc="$(./contrib/guix/guix-mk-distsrc x86_64-linux 2>"$src/mk-distsrc.log" | tail -n1)"
    ./contrib/guix/guix-build \
      --guix-system x86_64-linux \
      --target x86_64-unknown-linux-gnu \
      --package cuprated \
      --distsrc "$distsrc" >"$src/build.log" 2>&1

    artifact="$({ find contrib/guix/out -maxdepth 1 -type f -name 'cuprated-*-x86_64-unknown-linux-gnu.tar.gz' | LC_ALL=C sort | tail -n1; })"
    sha256sum "$artifact" | awk '{print $1}'
  )
}

# Mechanical regression guard: native-arch flags should never appear in the
# build log. Catches both the obvious (-march=native, -mcpu=native) and the
# rustc form (target-cpu=native) that any future cc-rs crate or rustc config
# could re-introduce.
assert_no_native_flags() {
  local src="$1"
  local pat='-march=native|-mcpu=native|target-cpu=native'
  if grep -E "$pat" "$src/contrib/guix/out/build-x86_64-unknown-linux-gnu.log" >/dev/null 2>&1; then
    echo "FAIL: host-CPU-native build flag detected in $src build log:" >&2
    grep -nE "$pat" "$src/contrib/guix/out/build-x86_64-unknown-linux-gnu.log" >&2 | head -5
    return 1
  fi
}

h1="$(run_once "$tmp/a")"
h2="$(run_once "$tmp/b")"

# Compare every reproducibility-sensitive output, not just the final binary.
compare() {
  local rel="$1"
  local a="$tmp/a/contrib/guix/out/$rel"
  local b="$tmp/b/contrib/guix/out/$rel"
  if [[ ! -f "$a" || ! -f "$b" ]]; then
    echo "FAIL: missing comparison file '$rel' in one of the runs" >&2
    return 1
  fi
  if ! cmp -s "$a" "$b"; then
    echo "FAIL: '$rel' differs between runs:" >&2
    diff -u "$a" "$b" | head -50 >&2 || true
    return 1
  fi
}

distsrc_basename="$(basename "$(find "$tmp/a/contrib/guix/out" -maxdepth 1 -type f -name 'cuprate-*-src.tar.gz' | head -n1)")"
artifact_basename="cuprated-$(awk -F'\"' '/"version"/{print $4; exit}' "$tmp/a/contrib/guix/out/build-metadata.json")-x86_64-unknown-linux-gnu.tar.gz"

compare "$distsrc_basename"
compare "$artifact_basename"
compare "build-metadata.json"
compare "rustc-version.txt"
compare "cargo-version.txt"
compare "guix-describe.json"

assert_no_native_flags "$tmp/a"
assert_no_native_flags "$tmp/b"

[[ "$h1" == "$h2" ]] || { echo "FAIL: final artifact sha mismatch ($h1 vs $h2)" >&2; exit 1; }
echo "reproducibility smoke test: PASS ($h1)"
