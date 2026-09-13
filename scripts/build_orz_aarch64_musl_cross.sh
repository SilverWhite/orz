#!/usr/bin/env bash
# orz-bin aarch64-unknown-linux-musl（静态）交叉编译入口 —— 0y NP1 安卓载体
# （2026-09-13；与 x86_64 S3 同纪律：源冻结 + 三件套 + 哈希/ELF/冒烟入记录）。
#
# 路线（沿用 2026-09-01「Linux arm 干跑」定案，非 QEMU 模拟）：
#   x86_64 rust 容器原生 rustc + zig cc 交叉编译 C 依赖 + rust-lld 链接。
#   - zig cc 只编译、不链接：zig 会注入自己的 crt1.o，与 rust self-contained
#     crt 冲突（duplicate symbol _start，2026-09-01 实测）。
#   - 链接用 rustup 自带 rust-lld（wrapper 剥离 `-Wl,` 前缀、过滤
#     `-nostartfiles`/`-nodefaultlibs`、显式 `-flavor gnu`）。
#   - 体量：QEMU 模拟 arm64 容器全量编译 3h+ 未完成；本路线实测 7m37s（-j 8）。
#
# ORZ-BUILD-MOUNT-001（2026-08-17）：父仓必须挂 /orz、工作区 /orz/orz。
#   orz-assurance 以 include_str! 从 CARGO_MANIFEST_DIR 读 ../../../runtime/*.json。
#
# 用法（宿主 PowerShell 调用；/out 直出载体目录）：
#   docker run --rm \
#     -v D:/CLI:/orz \
#     -v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml \
#     -v D:/tb-eval/zig-linux-014:/zig \
#     -v D:/tb-eval/orz-linux-arm:/out \
#     -w /orz/orz -e ORZ_BUILD_JOBS=4 \
#     rust:1.97-slim bash /orz/scripts/build_orz_aarch64_musl_cross.sh
#
# 可选回退：若 orz-tools build.rs 的官方 ripgrep aarch64 静态资产下载失败，
#   把该资产挂到容器并把路径经 ORZ_RG_AARCH64_STATIC 传入，本脚本会改用它
#   （`GROK_TOOLS_BUNDLE_RG_PATH`；build.rs 对动态链接件有 fail-fast 守卫）。
set -euo pipefail

if [ ! -f /orz/runtime/candidate-prefilter-config-v0.1.json ] || [ ! -f /orz/orz/Cargo.toml ]; then
  echo "FATAL: wrong container mount (ORZ-BUILD-MOUNT-001)" >&2
  echo "  mount the parent repo at /orz and run with -w /orz/orz" >&2
  exit 1
fi
[ -x /zig/zig ] || { echo "FATAL: zig not mounted at /zig/zig" >&2; exit 1; }

export RUSTUP_UPDATE_ROOT=https://static.rust-lang.org/rustup
export RUSTUP_DIST_SERVER=https://static.rust-lang.org

# apt：优先 aliyun 镜像（2026-09-01 实测可用），失败回退官方 HTTPS 源。
if [ -f /etc/apt/sources.list.d/debian.sources ]; then
  sed -i 's|deb.debian.org|mirrors.aliyun.com|g' /etc/apt/sources.list.d/debian.sources || true
fi
for f in /etc/apt/sources.list /etc/apt/sources.list.d/*; do
  [ -f "$f" ] && sed -i 's|http://deb.debian.org|https://deb.debian.org|g' "$f" || true
done

ok=0
for i in 1 2 3 4 5; do
  if apt-get update -qq && apt-get install -y -qq musl-tools protobuf-compiler make xz-utils; then
    ok=1
    break
  fi
  echo "apt attempt $i failed; sleeping 10s" >&2
  sleep 10
done
[ "$ok" = 1 ] || { echo "FATAL: apt update+install failed after 5 tries" >&2; exit 1; }

rustup target add aarch64-unknown-linux-musl

# ---- zig cc（C 依赖）+ rust-lld（链接）------------------------------------
export ZIG=/zig/zig
export ZIG_BIN=/zig/zig
export ZIG_TARGET=aarch64-linux-musl
chmod +x "$ZIG"
"$ZIG" version

cat > /tmp/zig-cc-wrapper.sh <<'WRAPPER_EOF'
#!/usr/bin/env bash
# zig cc wrapper —— 把 rust/cc-rs 的 triple 翻译成 zig 原生 target
# （cc-rs 传 --target=aarch64-unknown-linux-musl；zig 只接受 aarch64-linux-musl）。
set -euo pipefail
ZIG_BIN="${ZIG_BIN:-/zig/zig}"
ZIG_TARGET="${ZIG_TARGET:-aarch64-linux-musl}"
ARGS=()
has_target=0
for a in "$@"; do
  case "$a" in
    --target=aarch64-unknown-linux-musl)
      ARGS+=("-target" "aarch64-linux-musl"); has_target=1 ;;
    --target=aarch64-unknown-linux-gnu)
      ARGS+=("-target" "aarch64-linux-gnu"); has_target=1 ;;
    *) ARGS+=("$a") ;;
  esac
done
if [ "$has_target" -eq 0 ]; then
  # 链接阶段 rustc 不传 --target；不补会让 zig 用宿主 x86_64 lld 链 aarch64 目标文件。
  ARGS=("-target" "$ZIG_TARGET" "${ARGS[@]}")
fi
exec "$ZIG_BIN" cc "${ARGS[@]}"
WRAPPER_EOF
chmod +x /tmp/zig-cc-wrapper.sh

cat > /tmp/rust-lld-wrapper.sh <<'WRAPPER_EOF'
#!/usr/bin/env bash
# rust-lld wrapper —— 剥离 rustc link-arg 的 `-Wl,` 前缀（rust-lld 不认），
# 过滤 musl self-contained 用的 gcc/clang 参数，显式 -flavor gnu。
set -euo pipefail
ARGS=("-flavor" "gnu")
for a in "$@"; do
  case "$a" in
    -nostartfiles|-nodefaultlibs) ;;
    -Wl,*)
      inner="${a#-Wl,}"
      IFS=',' read -r -a parts <<< "$inner"
      for p in "${parts[@]}"; do ARGS+=("$p"); done ;;
    *) ARGS+=("$a") ;;
  esac
done
SYSROOT="$(rustc --print sysroot)"
RUST_LLD="$(find "$SYSROOT/lib/rustlib" -type f -name 'rust-lld' | head -1)"
[ -n "$RUST_LLD" ] || { echo "rust-lld-wrapper: cannot locate rust-lld" >&2; exit 1; }
exec "$RUST_LLD" "${ARGS[@]}"
WRAPPER_EOF
chmod +x /tmp/rust-lld-wrapper.sh

export CC_aarch64_unknown_linux_musl=/tmp/zig-cc-wrapper.sh
export CXX_aarch64_unknown_linux_musl=/tmp/zig-cc-wrapper.sh
export AR_aarch64_unknown_linux_musl="$ZIG ar"
export RANLIB_aarch64_unknown_linux_musl="$ZIG ranlib"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=/tmp/rust-lld-wrapper.sh

# ---- 打包进 orz-tools 的静态 ripgrep --------------------------------------
# 默认不设 GROK_TOOLS_BUNDLE_RG_PATH：cargo install ripgrep 在 aarch64-musl 下
# 因 tikv-jemalloc-sys 的 C 编译失败（atomic_memory_order_* undeclared，
# 2026-09-01 实测），改由 build.rs 自动下载官方 ripgrep 15.0.0
# aarch64-unknown-linux-gnu 资产（该资产本身为 musl 静态）。
if [ -n "${ORZ_RG_AARCH64_STATIC:-}" ] && [ -x "${ORZ_RG_AARCH64_STATIC}" ]; then
  export GROK_TOOLS_BUNDLE_RG_PATH="${ORZ_RG_AARCH64_STATIC}"
  echo "rg override: ${GROK_TOOLS_BUNDLE_RG_PATH}"
fi

# ---- 构建 -----------------------------------------------------------------
# target 放容器内：跨盘 bind mount 的指纹会失效导致全量重编（2026-09-01 实测）。
CARGO_TARGET_DIR=/tmp/orz-target \
  cargo build --release --target aarch64-unknown-linux-musl -p orz-bin \
  -j "${ORZ_BUILD_JOBS:-4}"

cp /tmp/orz-target/aarch64-unknown-linux-musl/release/orz /out/orz
cp /tmp/orz-target/aarch64-unknown-linux-musl/release/orz-signer /out/orz-signer
cp /tmp/orz-target/aarch64-unknown-linux-musl/release/orz-acaf-provision /out/orz-acaf-provision
ls -la /out/
