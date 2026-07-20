#!/usr/bin/env bash
set -euo pipefail

version="$1"
rust_target="$2"
source_date_epoch="$3"
out_dir="$4"
src_dir="$5"

stage_dir="$(mktemp -d)"
trap 'rm -rf "$stage_dir"' EXIT

umask 0022

binary="$src_dir/target/${rust_target}/release/cuprated"
install -m 0755 "$binary" "$stage_dir/cuprated"
install -m 0644 "$src_dir/LICENSE-AGPL" "$stage_dir/LICENSE-AGPL"
install -m 0644 "$src_dir/binaries/cuprated/cuprated.service" "$stage_dir/cuprated.service"

archive="cuprated-${version}-${rust_target}.tar.gz"
(
  cd "$stage_dir"
  tar \
    --sort=name \
    --mtime="@${source_date_epoch}" \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    --mode='a=rX,u+w' \
    --pax-option=exthdr.name=%d/PaxHeaders/%f,delete=atime,delete=ctime \
    -cf - cuprated LICENSE-AGPL cuprated.service \
  | gzip -n > "$out_dir/$archive"
)

(
  cd "$out_dir"
  sha256sum "$archive" > "$archive.SHA256SUM"
)
