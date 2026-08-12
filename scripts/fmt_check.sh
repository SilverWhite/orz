#!/usr/bin/env bash
# Formatting gate - fails when `cargo fmt --all -- --check` reports drift.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
echo "rustfmt check passed."
