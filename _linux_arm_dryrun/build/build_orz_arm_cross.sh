#!/usr/bin/env bash
# orz-bin Linux ARM64 (musl static) 交叉编译 — x86 rust 容器 + zig cc linker。
# 背景：QEMU 模拟 arm64 容器全量编译 orz 耗时 3h+ 且 orz 工作区大 crate 未开始；
# 改为 x86 宿主原生 rustc（快数十倍），C 依赖（aws-lc/zstd 等）由 zig cc
# 交叉编译为 aarch64-unknown-linux-musl 静态。
#
# 前置（已就绪，2026-09-01）：
#   - rust:1.97-slim 容器；rustup target add aarch64-unknown-linux-musl（容器内）。
#   - /zig 挂载 Linux x86_64 zig 0.14.1（D:/tb-eval/zig-linux-014；0.15 同 target
#     解析行为，0.14 实测通过）。
#   - /build 挂载 D:/CLI/_linux_arm_dryrun/build（含 zig-cc-wrapper.sh）。
#   - orz/.cargo/config.toml 已含 aarch64-unknown-linux-musl rustflags
#     （RELRO/NX/force-unwind）与 jemalloc 环境（BACKGROUND_THREADS=0, LG_PAGE=16）。
#
# ORZ-BUILD-MOUNT-001（2026-08-17）：父仓必须挂 /orz、工作区 /orz/orz。
set -euo pipefail

if [ ! -f /orz/runtime/candidate-prefilter-config-v0.1.json ] || [ ! -f /orz/orz/Cargo.toml ]; then
  echo "FATAL: wrong container mount (ORZ-BUILD-MOUNT-001)" >&2
  exit 1
fi

export RUSTUP_UPDATE_ROOT=https://static.rust-lang.org/rustup
export RUSTUP_DIST_SERVER=https://static.rust-lang.org

sed -i 's|deb.debian.org|mirrors.aliyun.com|g' /etc/apt/sources.list.d/debian.sources
apt-get update -qq
apt-get install -y -qq musl-tools protobuf-compiler make xz-utils

rustup target add aarch64-unknown-linux-musl

# zig cc 作为 aarch64-musl 的 C 编译器；wrapper 翻译 rust triple。
export ZIG=/zig/zig
chmod +x "$ZIG" /build/zig-cc-wrapper.sh
"$ZIG" version
WRAPPER=/build/zig-cc-wrapper.sh

# cargo 环境：CC/AR 指向 zig（编译 C 依赖）；**链接用 rust-lld**——
# zig cc 对 musl target 会注入自己的 crt1.o，与 rust self-contained crt1.o
# 冲突（duplicate symbol _start），即使 rustc 传 -nostartfiles 也无法抑制
# （zig 0.14 实测）。rust-lld 是 rustup 自带链接器，rustc 对名为 rust-lld 的
# linker 自动追加 -flavor gnu，能正确处理 musl self-contained crt。
export CC_aarch64_unknown_linux_musl="$WRAPPER"
export CXX_aarch64_unknown_linux_musl="$WRAPPER"
export AR_aarch64_unknown_linux_musl="$ZIG ar"
export RANLIB_aarch64_unknown_linux_musl="$ZIG ranlib"
cat > /tmp/rust-lld-wrapper.sh <<'WRAPPER_EOF'
#!/usr/bin/env bash
# rust-lld wrapper — 剥离 rustc -C link-arg 的 -Wl, 前缀并过滤 gcc/clang 参数。
set -euo pipefail
ARGS=("-flavor" "gnu")
for a in "$@"; do
  case "$a" in
    -nostartfiles|-nodefaultlibs)
      ;;
    -Wl,*)
      inner="${a#-Wl,}"
      IFS=',' read -r -a parts <<< "$inner"
      for p in "${parts[@]}"; do
        ARGS+=("$p")
      done
      ;;
    *)
      ARGS+=("$a")
      ;;
  esac
done
SYSROOT="$(rustc --print sysroot)"
RUST_LLD="$(find "$SYSROOT/lib/rustlib" -type f -name 'rust-lld' | head -1)"
if [ -z "$RUST_LLD" ]; then
  echo "rust-lld-wrapper: cannot locate rust-lld under $SYSROOT" >&2
  exit 1
fi
exec "$RUST_LLD" "${ARGS[@]}"
WRAPPER_EOF
chmod +x /tmp/rust-lld-wrapper.sh
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="/tmp/rust-lld-wrapper.sh"
export ZIG_BIN="$ZIG"
export ZIG_TARGET="aarch64-linux-musl"

# target 放容器内（避免跨盘 bind mount 的 fingerprint 失效导致全量重编）。
CARGO_TARGET_DIR=/tmp/orz-target cargo build --release --target aarch64-unknown-linux-musl -p orz-bin -j 8

cp /tmp/orz-target/aarch64-unknown-linux-musl/release/orz /out/orz
cp /tmp/orz-target/aarch64-unknown-linux-musl/release/orz-signer /out/orz-signer
cp /tmp/orz-target/aarch64-unknown-linux-musl/release/orz-acaf-provision /out/orz-acaf-provision
ls -la /out/
