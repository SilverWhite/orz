#!/usr/bin/env bash
# zig cc wrapper — 把 rust/cc-rs 的 triple 格式翻译成 zig 原生 target。
# cc-rs 会传 --target=aarch64-unknown-linux-musl（rust 标准 triple），
# zig 只接受 aarch64-linux-musl（无 unknown 字段）。
set -euo pipefail

ZIG_BIN="${ZIG_BIN:-/zig/zig}"
ZIG_TARGET="${ZIG_TARGET:-aarch64-linux-musl}"
ARGS=()
has_target=0
for a in "$@"; do
  case "$a" in
    --target=aarch64-unknown-linux-musl)
      ARGS+=("-target" "aarch64-linux-musl")
      has_target=1
      ;;
    --target=aarch64-unknown-linux-gnu)
      ARGS+=("-target" "aarch64-linux-gnu")
      has_target=1
      ;;
    *)
      ARGS+=("$a")
      ;;
  esac
done
if [ "$has_target" -eq 0 ]; then
  # rustc 链接阶段不传 --target 给 linker；显式补上，否则 zig 用宿主
  # x86_64 lld 链接 aarch64 目标文件（is incompatible with elf64-x86-64）。
  ARGS=("-target" "$ZIG_TARGET" "${ARGS[@]}")
fi
exec "$ZIG_BIN" cc "${ARGS[@]}"
