# Guix reproducible build flow (cuprated)

This directory provides a Guix-first reproducible release pipeline for
`cuprated` on Linux. The goal is that anyone with the right toolchain
can rebuild the published `cuprated-<version>-x86_64-unknown-linux-gnu.tar.gz`
from this repository and get **byte-identical** output, then verify the
release's SHA256 against the upstream publication.

## Scope

| | |
|---|---|
| Supported target today | `x86_64-unknown-linux-gnu` |
| Supported Guix system  | `x86_64-linux`           |
| Other architectures    | not yet — see [Roadmap](#roadmap) |

This is a node binary, not a miner. RandomX is built in CMake's
`ARCH=default` mode (compiler-capability-gated `-maes -mssse3 -mavx2`,
no `-march=native`), so the binary is identical across x86_64 CPUs
under the same toolchain. Block-verification uses RandomX's "light"
mode, which doesn't depend on CPU-specific codegen for correctness;
miners using `randomx-rs` directly should override via the
`RANDOMX_ARCH` env var to opt in to host-specific instructions.

## Build flow

1. Create a deterministic source archive (vendored Cargo deps, fixed
   mtimes/uid/gid/path-prefix), inside a hermetic Guix shell:

   ```bash
   ./contrib/guix/guix-mk-distsrc x86_64-linux
   ```

2. Build `cuprated` from the source archive using `guix time-machine`
   pinned to the channel in `channels.scm`:

   ```bash
   ./contrib/guix/guix-build \
     --guix-system x86_64-linux \
     --target x86_64-unknown-linux-gnu \
     --package cuprated \
     --distsrc contrib/guix/out/cuprate-<version>-<commit>-src.tar.gz
   ```

3. Aggregate checksums + GPG-signed JSON attestation:

   ```bash
   ./contrib/guix/guix-checksums contrib/guix/out
   ./contrib/guix/guix-attest    contrib/guix/out <builder-id> <version>
   ```

4. Sidecar SHA256 integrity check against a published `.SHA256SUM`:

   ```bash
   ./contrib/guix/guix-verify contrib/guix/out/cuprated-<version>-x86_64-unknown-linux-gnu.tar.gz
   ```

5. End-to-end reproducibility self-check (builds twice and compares
   every determinism-sensitive output):

   ```bash
   ./contrib/guix/smoke-reproducible.sh
   ```

### Concurrency

Each script uses per-run `mktemp` working directories, so concurrent
invocations cannot trample each other's *intermediate* state. **Final
outputs share fixed names by default** (e.g. `cuprated-<version>-<target>.tar.gz`,
`build-metadata.json`, `build-<target>.log`), so two concurrent runs
writing into the same `contrib/guix/out` would overwrite each other's
results. Use `--out-dir <path>` (on `guix-mk-distsrc` and `guix-build`)
to give each parallel run its own output directory; the path is
required to live inside the repository so the Guix container can see
it. `smoke-reproducible.sh` sidesteps this entirely by performing each
run in its own `git clone` of the working tree.

## Output files

`contrib/guix/out/` after a full run:

- `cuprate-<version>-<commit>-src.tar.gz` (+ `.SHA256SUM`) — deterministic source
- `cuprated-<version>-<rust-target>.tar.gz` (+ `.SHA256SUM`) — release artifact
- `SHA256SUMS` — aggregate sum over the two tarballs only
- `build-metadata.json` — package, version, target, `SOURCE_DATE_EPOCH`,
  git commit, distsrc sha256, RandomX arch mode
- `guix-describe.json` — channel + commit metadata for the Guix instance
  used to build (captured on the *outer* host before entering the
  container, since the container doesn't ship `guix` itself)
- `rustc-version.txt`, `cargo-version.txt` — toolchain proof
- `build-<rust-target>.log` — full verbose build log (used by the
  smoke test's native-flag regression guard)
- `ldd-<rust-target>.diag.txt` — **diagnostic only**; host-loader output
  varies by system and is intentionally excluded from `SHA256SUMS` and
  the signed attestation

## Determinism inputs

Everything that goes into a build is pinned at one of these layers:

| Layer | Pin | Where |
|---|---|---|
| Guix instance | commit sha + channel introduction | `channels.scm` |
| Build profile (rust, gcc-toolchain, cmake, make, openssl, …) | by Guix package name; transitively pinned via Guix commit | `manifest.scm` |
| Rust source tree | git commit, verified at build time against `git archive` of the same commit (`libexec/build.sh`'s distsrc-equivalence check) | `.cuprate-distsrc.json` |
| Rust deps | `Cargo.lock`, then `cargo vendor --locked --versioned-dirs` | `Cargo.lock`, `vendor/` |
| C/C++ build flags | `--remap-path-prefix`, `-ffile-prefix-map`, `-C codegen-units=1` | `libexec/build.sh` |
| Time | `SOURCE_DATE_EPOCH` = git-commit time | `libexec/build.sh` |
| Tar metadata | `--sort=name --mtime=@EPOCH --owner=0 --group=0 --numeric-owner --mode='a=rX,u+w'` | `mk-distsrc`, `libexec/package.sh` |
| Gzip metadata | `gzip -n` (no name, no timestamp) | same |

## Distsrc content equivalence

`libexec/build.sh` does **not** trust `.cuprate-distsrc.json`'s
`git_commit` field on its own. After extracting the distsrc tarball,
it runs `git archive` on that commit from the outer checkout. The
three paths `mk-distsrc` legitimately adds (`vendor/`, `.cargo/`,
`.cuprate-distsrc.json`) are moved aside to a temporary directory
under `$build_root`, then `diff -rq` runs over the rest of the tree
with no excludes — so a nested file or directory that happens to
share one of those basenames (a hypothetical future `tests/vendor/`,
for example) is still compared. After the diff succeeds, the three
paths are restored so the build can use them; on diff failure, the
build root is preserved with the paths restored so the operator can
inspect the as-extracted distsrc. A distsrc that claims a git commit
but contains modified sources fails this check before the build
starts.

## Threat model

**Trust roots** (compromise of any of these defeats reproducibility
verification):

1. **The Guix substitute key set authorised on the building host.**
   Guix accepts substitutes only when they validate under one of the
   keys passed to `guix archive --authorize`. The CI workflow loads
   those keys from the verified Guix binary tarball (see #2 below);
   any host with a substitute key controls what binary artifacts the
   Guix daemon will accept as cached toolchain pieces.
2. **The Guix binary bootstrap tarball.** The CI workflow downloads
   `guix-binary-<ver>.<arch>.tar.xz` from `ftp.gnu.org`, then verifies
   it against both a pinned SHA256 *and* a pinned OpenPGP fingerprint
   (`A28BF40C…3D8351`, Efraim Flashner). Compromise of *both* trust
   anchors at the same time would let a malicious tarball through.
3. **The Guix channel commit + introduction.** `channels.scm` pins the
   channel by commit sha and supplies the canonical
   `make-channel-introduction` for the official Guix channel. `guix
   time-machine` authenticates the channel head against that
   introduction before evaluating anything.
4. **The release signing key + publication channel.** Reproducibility
   proves *what got shipped matches the source*; it does not prove
   that the shipped binary is the one users should run. Users still
   need to verify the published `cuprated-*.tar.gz.SHA256SUM` (or the
   signed JSON attestation under `contrib/guix/sigs/`) against
   maintainer-controlled distribution.
5. **The git tree itself.** A malicious commit landing on `main`
   yields a malicious-but-reproducibly-built binary. Reproducibility
   shifts the trust requirement onto code review, not away from it.

**This pipeline protects against:**

- A future toolchain regression silently changing artifact contents
  (Guix pin + lockfile + vendor make any drift a load-bearing diff).
- A maintainer (or attacker with access to one) shipping a binary
  that doesn't correspond to the published source.
- Supply-chain bit-rot in transitive Cargo deps — the lockfile +
  vendor freezes everything; new upstream versions only enter via a
  reviewable commit.
- A tampered distsrc tarball that claims a git commit but contains
  modified sources (see [Distsrc content
  equivalence](#distsrc-content-equivalence)).

**This pipeline does NOT protect against:**

- Compromise of any of the trust roots listed above.
- Hardware-level attacks on the build host (compromised microcode,
  flashed firmware, etc.).
- A bug in `cuprated` itself. Reproducibility is about *what got
  shipped*, not whether what got shipped is correct.

## RandomX

This pipeline depends on `randomx-rs` building RandomX in a
host-CPU-independent mode. As of `Cuprate/randomx-rs@567bdca`, this
happens **by accident** — a useful one to be aware of:

- `randomx-rs/build.rs` calls `cmake::Config::new(…).define("DARCH",
  "native")`, which emits `-DDARCH=native` to CMake.
- Upstream [`tevador/RandomX`](https://github.com/tevador/RandomX)'s
  `CMakeLists.txt` has **zero** references to `DARCH`. It reads
  `ARCH`, which defaults to `"default"` when unset.
- So the `.define("DARCH", …)` line is a years-old silent typo. The
  actual ARCH value that takes effect is the CMake default,
  `"default"`, regardless of what `DARCH` is set to.
- `ARCH="default"` produces a build with `-maes -mssse3 -mavx2`
  flags, but each of those is gated on a *compiler-capability* check
  (`check_c_compiler_flag`), not on the build host's CPU. With the
  Guix toolchain pinned, all three flags resolve identically across
  hosts, so the produced object code is identical.

If `randomx-rs` ever corrects the typo (legitimately, since the line
is meaningless as written), the *named-by-intent* behaviour
(`-march=native`) would kick in and the build would silently stop
being reproducible across CPUs. The CI smoke job grep-fails on
`-march=native` / `-mcpu=native` / `target-cpu=native` to catch that
regression before merge.

Miners using `randomx-rs` directly and wanting host-specific
performance can set `RANDOMX_ARCH=native` in their environment — but
that has no effect today because of the same typo. Filing a one-line
fix upstream at `Cuprate/randomx-rs` would unblock both that and our
regression guard simultaneously.

## Known workarounds (remove when upstream fixes land)

- **`_GLIBCXX_HAVE_FENV_H` / `_GLIBCXX_USE_C99_FENV` defined in
  `CXXFLAGS`** (`libexec/build.sh`). Guix's gcc-15.2 libstdc++ ships
  with these undefined in `bits/c++config.h`, so `<cfenv>` doesn't
  pull in `<fenv.h>` and `fesetround` is absent from the global
  namespace. Any C++ caller of `<cfenv>` (RandomX,
  `src/instructions_portable.cpp`) then fails to link. Setting both
  macros restores the expected `<cfenv>` behaviour. To check whether
  this is still needed, run with `GUIX_SKIP_FENV_WORKAROUND=1` and
  see if RandomX still compiles.

- **`OPENSSL_NO_VENDOR=1`** to force `openssl-sys` to link the audited
  openssl from `manifest.scm` instead of recompiling its bundled
  `openssl-src` from source. This is a behavioural improvement, not a
  workaround for a bug; it's recorded here as a knob.

## CI

`.github/workflows/guix-reproducibility.yml` runs `smoke-reproducible.sh`
on every PR that touches `contrib/guix/**`, `Cargo.toml`, `Cargo.lock`,
or the workflow itself. It can also be triggered manually via
`workflow_dispatch` for PRs that change crate source without touching
the pipeline, and runs weekly on the default branch via `schedule` to
catch drift those source-only PRs would otherwise miss. The workflow:

- pins `actions/checkout` by commit sha (not tag)
- pins the Guix binary tarball by SHA256 AND by GPG signer
  fingerprint, and fails before extracting if either check fails
- runs the smoke script, which itself fails on any divergence in
  distsrc / artifact / metadata / toolchain versions / guix-describe
  output, and on any `-march=native` / `-mcpu=native` /
  `target-cpu=native` flag appearing in the build log

## Roadmap

- aarch64-linux target
- macOS (`x86_64-darwin`, `aarch64-darwin`) — needs an alternative
  hermetic-build story; Guix doesn't run natively on macOS
- A signed-attestation flow compatible with sigstore / SLSA
  provenance (the current `guix-attest` produces a stable, sorted-key
  signed JSON attestation; SLSA-format export is a future step)
- Re-pinning `channels.scm` to a tagged Guix release once one ships
  with rust ≥ 1.91 (v1.5.0 only carries 1.88; we currently pin to a
  recent commit on master)
