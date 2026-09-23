#!/usr/bin/env bash
# Compile sofui and a second GPUI application after copying only the library
# tree outside the SofDevTool workspace. This catches accidental root manifest,
# application/core crate, and resource-path dependencies before extraction.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
isolation_root="$(mktemp -d "${TMPDIR:-/tmp}/sofui-isolation.XXXXXX")"
trap 'rm -rf -- "$isolation_root"' EXIT

mkdir -p "$isolation_root/sofui/tests"
cp "$repo_root/crates/ui/Cargo.toml" "$repo_root/crates/ui/Cargo.lock" \
  "$repo_root/crates/ui/README.md" "$isolation_root/sofui/"
# Match the root workspace's debug profile in the temporary standalone build
# without putting a nested profile in sofui's reusable package manifest.
printf '\n[profile.dev]\nopt-level = 1\n' >> "$isolation_root/sofui/Cargo.toml"
cp -R "$repo_root/crates/ui/src" "$repo_root/crates/ui/examples" \
  "$isolation_root/sofui/"
mkdir -p "$isolation_root/sofui/tests/consumer"
cp "$repo_root/crates/ui/tests/consumer/Cargo.toml" \
  "$repo_root/crates/ui/tests/consumer/Cargo.lock" \
  "$isolation_root/sofui/tests/consumer/"
cp -R "$repo_root/crates/ui/tests/consumer/src" \
  "$isolation_root/sofui/tests/consumer/"
export CARGO_NET_OFFLINE=true
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target}"

echo "== isolated sofui package and gallery =="
cargo build --offline --locked --manifest-path "$isolation_root/sofui/Cargo.toml" --example gallery

echo "== isolated independent GPUI consumer =="
cargo build --offline --locked --manifest-path "$isolation_root/sofui/tests/consumer/Cargo.toml"

echo "sofui isolation passed"
