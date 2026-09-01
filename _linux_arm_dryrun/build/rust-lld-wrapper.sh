#!/usr/bin/env bash
# rust-lld wrapper — 剥离 rustc -C link-arg 的 -Wl, 前缀。
# orz/.cargo/config.toml 对 aarch64-unknown-linux-musl 注入
#   -C link-arg=-Wl,-z,relro,-z,now,-z,noexecstack
# rust-lld（直接调用时）不认识 -Wl, 前缀；剥离后转成原生 -z 参数。
set -euo pipefail

# rustc 只在 linker 可执行文件名为 "rust-lld" 时自动追加 -flavor gnu；
# 本 wrapper 名不匹配，须显式指定。
ARGS=("-flavor" "gnu")
for a in "$@"; do
  case "$a" in
    -nostartfiles|-nodefaultlibs)
      # rustc 为 musl self-contained 传的 gcc/clang 参数；rust-lld 不识别
      # 且不需要（crt 由 rustc 显式传入）。
      ;;
    -Wl,*)
      inner="${a#-Wl,}"
      # 逗号分隔 -> 独立参数（-z relro 等）。
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

# rust-lld 不在 PATH：位于 rustc sysroot 的 lib/rustlib/<triple>/bin/ 下。
SYSROOT="$(rustc --print sysroot)"
RUST_LLD="$(find "$SYSROOT/lib/rustlib" -type f -name 'rust-lld' | head -1)"
if [ -z "$RUST_LLD" ]; then
  echo "rust-lld-wrapper: cannot locate rust-lld under $SYSROOT" >&2
  exit 1
fi
exec "$RUST_LLD" "${ARGS[@]}"
