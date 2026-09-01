#!/usr/bin/env bash
# Run the orz unit-test suite for the Linux musl target inside the build
# container (2026-08-27, THIN-HARNESS-REDESIGN 统一编译轮验证).
# Reuses /target (release deps cached by build_orz_aliyun_trixie.sh).
set -euo pipefail

export RUSTUP_UPDATE_ROOT=https://static.rust-lang.org/rustup
export RUSTUP_DIST_SERVER=https://static.rust-lang.org

apt-get update -qq
apt-get install -y -qq musl-tools protobuf-compiler make

rustup target add x86_64-unknown-linux-musl

# Static musl rg for tests that exercise the bundled search tool
# (GROK_TOOLS_BUNDLE_RG_PATH contract — see build script note).
cargo install ripgrep --version 15.0.0 --locked --target x86_64-unknown-linux-musl --root /tmp/rg-static
export GROK_TOOLS_BUNDLE_RG_PATH=/tmp/rg-static/bin/rg

CARGO_TARGET_DIR=/target cargo test --release --target x86_64-unknown-linux-musl \
  -p orz-assurance -p orz-loop -p orz-host -p orz-tui -p orz-bin -j 1
