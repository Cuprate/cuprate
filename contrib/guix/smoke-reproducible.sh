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

# Where to drop the per-run build logs so a later CI step (or a curious
# operator) can scan them. The smoke script is typically invoked under
# `sudo` in CI, which makes $tmp root-owned mode 0700 - so the later
# non-sudo workflow step that greps for native-arch flags can't read
# anything in /tmp/cuprate-smoke.*. Exporting the logs to a workspace-
# relative path and chmod a+rX fixes that for both success and failure
# without changing the failure-preserves-tmp behaviour.
LOG_EXPORT_DIR="${LOG_EXPORT_DIR:-$repo_root/contrib/guix/smoke-logs}"

# Preserve the working tree on failure so the user can dig through logs and
# intermediate artifacts. On success, clean up. Always export the build
# logs first so the assert step (in this script and at the workflow level)
# has something to scan even on success.
on_exit() {
  local code=$?
  if [[ -d "$tmp" ]]; then
    mkdir -p "$LOG_EXPORT_DIR"
    local sub
    for sub in a b; do
      local src_log="$tmp/$sub/contrib/guix/out/build-x86_64-unknown-linux-gnu.log"
      if [[ -f "$src_log" ]]; then
        cp "$src_log" "$LOG_EXPORT_DIR/build-$sub.log" 2>/dev/null || true
      fi
    done
    chmod -R a+rX "$LOG_EXPORT_DIR" 2>/dev/null || true
  fi
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
#
# NOTE: the `--` before "$pat" is load-bearing. The regex begins with `-m`,
# which grep otherwise parses as the `-m` (max-count) option, resulting in
# `grep: invalid max count` and a non-zero exit that the `if` block
# silently treats as "not found" - i.e. the guard becomes fail-open.
NATIVE_FLAG_REGEX='-march=native|-mcpu=native|target-cpu=native'
assert_no_native_flags() {
  local src="$1"
  local log="$src/contrib/guix/out/build-x86_64-unknown-linux-gnu.log"
  if [[ ! -s "$log" ]]; then
    echo "FAIL: build log is missing or empty: $log" >&2
    return 1
  fi
  if grep -E -- "$NATIVE_FLAG_REGEX" "$log" >/dev/null 2>&1; then
    echo "FAIL: host-CPU-native build flag detected in $src build log:" >&2
    # `head -5 >&2`: redirect head's output to stderr, not grep's
    # (otherwise head reads from an empty pipe and prints nothing).
    grep -nE -- "$NATIVE_FLAG_REGEX" "$log" | head -5 >&2
    return 1
  fi
}

# Positive self-test for the regex/guard: a known-bad line MUST trip the
# grep. If this ever stops firing, the regression guard is broken (the
# leading `-m` parses as an option without `--`/`-e`).
native_flag_selftest() {
  local tmpfile
  tmpfile="$(mktemp "${TMPDIR:-/tmp}/native-flag-selftest.XXXXXX")"
  printf '%s\n' 'cc -march=native foo.c' > "$tmpfile"
  if ! grep -E -- "$NATIVE_FLAG_REGEX" "$tmpfile" >/dev/null 2>&1; then
    echo "FAIL: native-flag guard self-test failed; regex is broken" >&2
    rm -f "$tmpfile"
    return 1
  fi
  rm -f "$tmpfile"
}

native_flag_selftest

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
