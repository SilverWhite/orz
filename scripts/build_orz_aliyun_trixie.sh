#!/usr/bin/env bash
# Build orz-bin for Linux (musl static) — trixie-compatible variant of
# build_orz_aliyun.sh (2026-08-27). rust:1.97-slim 已换基到 Debian trixie，
# mirrors.aliyun.com/debian 无 trixie 目录（404），故跳过 aliyun 替换、
# 直连官方 deb.debian.org（容器内实测可达，musl-tools candidate 1.2.5-3.1）。
# Mounts: /orz (source), /target (CARGO_TARGET_DIR), /out (binary output).
# 2026-08-17 (GAP-ACAF-HARNESS-PASSTHROUGH): also copy orz-signer and
# orz-acaf-provision so the eval container can materialise the ACAF
# manifest/keystore at install time.
set -euo pipefail

# 2026-09-07: Clash TUN 环境下 http://deb.debian.org (80) 经代理节点回
# 502 Bad Gateway / InRelease 签名不可达；HTTPS (443) 实测可用 —— apt 源
# 强制 https（工具链网络适配，不触及 orz 源）。
for f in /etc/apt/sources.list /etc/apt/sources.list.d/*; do
  if [ -f "$f" ]; then
    sed -i 's|http://deb.debian.org|https://deb.debian.org|g' "$f" || true
  fi
done

# ORZ-BUILD-MOUNT-001 (2026-08-17): the parent repo MUST be mounted at /orz
# and the workspace at /orz/orz — orz-assurance embeds ../../../runtime/*.json
# via include_str! resolved from CARGO_MANIFEST_DIR. A wrong mount fails here
# in <1s with the correct command instead of ~20min into the compile.
if [ ! -f /orz/runtime/candidate-prefilter-config-v0.1.json ] || [ ! -f /orz/orz/Cargo.toml ]; then
  echo "FATAL: wrong container mount (ORZ-BUILD-MOUNT-001)" >&2
  echo "  mount the parent repo at /orz and run with -w /orz/orz, e.g.:" >&2
  echo "  docker run --rm -v D:/CLI:/orz -v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml -v D:/tb-eval/orz-target:/target -v D:/tb-eval/orz-linux:/out -w /orz/orz rust:1.97-slim bash /build.sh" >&2
  exit 1
fi

# 2026-08-20: rsproxy.cn /dist/ TLS 失败、aliyun rustup manifest 过期，改用官方 static.rust-lang.org（容器内实测 200）。
export RUSTUP_UPDATE_ROOT=https://static.rust-lang.org/rustup
export RUSTUP_DIST_SERVER=https://static.rust-lang.org

# 2026-08-27 (trixie): mirrors.aliyun.com/debian 缺 trixie 目录（404）——
# 不再替换镜像，直连官方源（容器内实测 apt-get update 全绿）。
apt-get update -qq
apt-get install -y -qq musl-tools protobuf-compiler ripgrep make

rustup target add x86_64-unknown-linux-musl

# orz-tools bundles ripgrep. Do NOT point the override at the apt binary: it
# is glibc-dynamic (trixie, needs GLIBC_2.39) and cannot load in the task
# containers (bookworm glibc 2.36) — exits 1 with empty stdout, which grep
# used to mask as "No matches found" (FUS-TOOL-SCOPE-CONTRACT, 2026-08-17).
# Build a static musl rg from source instead (crates.io via rsproxy; the
# build.rs static-link guard fails fast on any dynamic override).
cargo install ripgrep --version 15.0.0 --locked --target x86_64-unknown-linux-musl --root /tmp/rg-static
export GROK_TOOLS_BUNDLE_RG_PATH=/tmp/rg-static/bin/rg

CARGO_TARGET_DIR=/target cargo build --release --target x86_64-unknown-linux-musl -p orz-bin -j 1

cp /target/x86_64-unknown-linux-musl/release/orz /out/orz
cp /target/x86_64-unknown-linux-musl/release/orz-signer /out/orz-signer
cp /target/x86_64-unknown-linux-musl/release/orz-acaf-provision /out/orz-acaf-provision
ls -la /out/
