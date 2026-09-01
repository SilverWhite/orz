#!/usr/bin/env bash
# Build orz-bin for Linux ARM64 (musl static) — Linux arm 干跑载体。
# Mirrors D:/tb-eval/build_orz_aliyun.sh with target switched to
# aarch64-unknown-linux-musl. Run on an arm64-emulated container so the
# native musl-gcc (aarch64) resolves link-time symbols correctly.
#
# ORZ-BUILD-MOUNT-001 (2026-08-17): the parent repo MUST be mounted at /orz
# and the workspace at /orz/orz — orz-assurance embeds ../../../runtime/*.json
# via include_str! resolved from CARGO_MANIFEST_DIR. A wrong mount fails here
# in <1s with the correct command instead of ~20min into the compile.
set -euo pipefail

if [ ! -f /orz/runtime/candidate-prefilter-config-v0.1.json ] || [ ! -f /orz/orz/Cargo.toml ]; then
  echo "FATAL: wrong container mount (ORZ-BUILD-MOUNT-001)" >&2
  echo "  mount the parent repo at /orz and run with -w /orz/orz, e.g.:" >&2
  echo "  docker run --rm --platform linux/arm64/v8 -v D:/CLI:/orz -v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml -v D:/tb-eval/orz-target-arm:/target -v D:/tb-eval/orz-linux-arm:/out -w /orz/orz rust:1.97-slim bash /build.sh" >&2
  exit 1
fi

export RUSTUP_UPDATE_ROOT=https://static.rust-lang.org/rustup
export RUSTUP_DIST_SERVER=https://static.rust-lang.org

# apt -> aliyun mirror (same semantics as build_orz_aliyun.sh).
sed -i 's|deb.debian.org|mirrors.aliyun.com|g' /etc/apt/sources.list.d/debian.sources
apt-get update -qq
apt-get install -y -qq musl-tools protobuf-compiler ripgrep make

rustup target add aarch64-unknown-linux-musl

# Static rg bundled into orz-tools (FUS-TOOL-SCOPE-CONTRACT,
# ORZ-TOOL-BINARY-COMPAT-001). Do NOT set GROK_TOOLS_BUNDLE_RG_PATH here:
# cargo-installing ripgrep for aarch64-unknown-linux-musl fails in
# tikv-jemalloc-sys C compilation (atomic_memory_order_* undeclared under
# aarch64-musl, observed 2026-09-01). orz-tools build.rs auto-downloads the
# official ripgrep 15.0.0 aarch64-unknown-linux-gnu asset (musl-static) when
# the override env is unset — verified supported by build.rs's asset triple
# map ("linux","aarch64" -> aarch64-unknown-linux-gnu).

CARGO_TARGET_DIR=/target cargo build --release --target aarch64-unknown-linux-musl -p orz-bin -j 1

cp /target/aarch64-unknown-linux-musl/release/orz /out/orz
cp /target/aarch64-unknown-linux-musl/release/orz-signer /out/orz-signer
cp /target/aarch64-unknown-linux-musl/release/orz-acaf-provision /out/orz-acaf-provision
ls -la /out/
