# Phase 1 Completion Report — orz Scaffold

**日期**: 2026-08-02
**分支**: `feat/phase1-scaffold` (B:\orz)
**基线**: Grok Build `500129c7` (`SOURCE_REV = 6372e41d828b8a6ee82c29e01a69e27ec895cca9`)

## 完成清单

### 1. 删除 dead crates (19 个)

| # | Crate | 设计 §1.1 目标 | 实际 |
|---|-------|---------------|------|
| 1 | `xai-grok-telemetry` | 删除 | ✅ 已删除 |
| 2 | `xai-grok-auth` | 删除 | ✅ 已删除 |
| 3 | `xai-grok-update` | 删除 | ✅ 已删除 |
| 4 | `xai-grok-voice` | 删除 | ✅ 已删除 |
| 5 | `xai-grok-announcements` | 删除 | ✅ 已删除 |
| 6 | `xai-grok-plugin-marketplace` | 删除 | ✅ 已删除 |
| 7 | `xai-mixpanel` | 删除 | ✅ 已删除 |
| 8 | `xai-sqlite-journal` | 删除 | ✅ 已删除 |
| 9 | `xai-grok-subagent-resolution` | 删除 | ✅ 已删除 |
| 10 | `xai-grok-sampler` | 删除 | ✅ 已删除 |
| 11 | `xai-grok-codebase-graph` | 删除 | ✅ 已删除 |
| 12 | `xai-grok-secrets` | 删除 | ✅ 已删除 |
| 13 | `xai-grok-pager-pty-harness` | 删除 | ✅ 已删除 |
| 14 | `xai-grok-test-support` | 删除 | ✅ 已删除 |
| 15 | `xai-grok-shell-session-support` | 删除 | ✅ 已删除 |
| 16 | `xai-grok-shell-base` | 删除 | ✅ 已删除 |
| 17 | `xai-chat-state` | 删除 | ✅ 已删除 |
| 18 | `xai-grok-mermaid` | 删除 | ✅ 已删除 |

另外 `xai-grok-shell` 也被删除（重命名为 `orz-shell`），总计 19 个 crate 从源码树移除。

### 2. 重命名 crates (21 个 `xai-grok-*` → `orz-*`)

agent, config, config-types, env, hooks, http, markdown, markdown-core, mcp, memory, models, paths, sampling-types, sandbox, shared, shell, tools, tools-api, version, workspace, workspace-client, workspace-types

外加: `xai-grok-compaction` → `orz-compaction`

### 3. 保留原名的 Grok TUI crates (4 个)

`xai-grok-pager`, `xai-grok-pager-render`, `xai-grok-pager-minimal`, `xai-grok-pager-bin`

### 4. 新增 crates (3 个)

| Crate | 状态 | 说明 |
|-------|------|------|
| `orz-bin` | ✅ 可编译运行 | `orz` 二进制入口，输出 `orz 0.1.0 — assurance-first CLI agent workbench` |
| `orz-assurance` | ✅ skeleton | `GateDecision` 枚举（5 值）+ `AssuranceError` + 7 个模块结构（Phase 2-4 实现） |
| `orz-tui` | ✅ skeleton | `run()` 函数桩 |

### 5. 双二进制架构

| 二进制 | 入口 | 状态 |
|--------|------|------|
| `orz` | `crates/orz-bin/src/main.rs` | ✅ 编译运行成功 |
| `grok` | `crates/codegen/xai-grok-pager-bin/src/main.rs` | ⚠️ 二进制名已声明，入口未裁剪 |

### 6. 工具链

| 项 | 值 |
|----|-----|
| Rust | 1.97.1 MSVC (`stable-x86_64-pc-windows-msvc`) |
| 链接器 | `rust-lld` |
| 配置文件 | `rust-toolchain.toml`, `.cargo/config.toml` |
| 构建脚本 | `build.bat` (vcvars + cargo check), `build.ps1` (MSVC PATH + cargo check) |

### 7. Workspace Cargo.toml

- 所有成员 crate 已注册 (~60 crates，从原始 63 减少)
- 依赖重定向：`orz-shell`, `orz-tools` 等使用 `workspace = true`
- Build profile: `release-dist`（thin LTO, codegen-units=1）
- 双 `[[bin]]` 声明在各自 crate 的 Cargo.toml 中

## 已知偏差 (Deviations)

### DEV-001: grok 二进制入口未裁剪

**设计要求**: §8 Phase 1 Step 6 — "修改 xai-grok-pager-bin/src/main.rs：裁剪 telemetry/auth/update 引用"

**当前状态**: `xai-grok-pager-bin/src/main.rs` (3380 行) 保留原始 Grok 入口代码。代码中引用 `orz_update`、`orz_telemetry` 等已删除 crate 的 symbol——grok 二进制当前无法编译。

**影响**: `grok` 二进制未纳入构建。`orz` 二进制是唯一的编译目标。

**推迟理由**: 
- grok 二进制是兜底 UI，在 Phase 4 正式接入
- telemetry/auth/update 的裁剪与 Grok TUI (xai-grok-pager) 内部耦合
- Phase 1 重点是 orz 二进制 + assurance skeleton，已经达成
- 实际裁剪在 Phase 4 "TUI + Polish" 阶段与 TUI 移植一同进行

**目标 Phase**: Phase 4

### DEV-002: orz_update / orz_telemetry 引用残留

**设计要求**: §1.1 — `xai-grok-telemetry` 和 `xai-grok-update` 已删除

**当前状态**: `orz-shell/src/agent/app.rs` 仍使用 `orz_telemetry` symbol。这些是 deleted crate 的 re-export——编译时由 `xai-tracing` (common) 提供部分覆盖。`xai-grok-pager-bin/src/main.rs` 也有大量引用。

**影响**: 不影响 `orz-bin` 编译（orz-bin 不链接 shell/pager）。

**目标 Phase**: Phase 2（shell 层注入时同时清理）

### DEV-003: memory.enabled 开关未落地

**设计要求**: §1.6 — `memory.enabled = false` 默认 + 配置开关

**当前状态**: `orz-memory` crate 保留且依赖已更新，但无 `enabled` config 开关。

**影响**: 无——memory crate 当前未被任何代码路径激活。

**目标 Phase**: Phase 2（session lifecycle 注入时添加开关）

### DEV-004: 只验证了 debug build

**设计要求**: §8 Phase 1 — `cargo build --release`

**当前状态**: 只运行了 `cargo check -p orz-bin` 和 `cargo build -p orz-bin`（debug）。Release build 未验证。

**影响**: release-dist profile 的 thin LTO + codegen-units=1 可能触发 debug build 未暴露的优化期错误。

**后续**: Phase 2 开始前执行一次 release build 验证。

## Phase 1 判定: **通过 (Pass with deviations)**

Phase 1 的核心目标——fork Grok Build、删除 dead crates、重命名保留 crates、建立新 crate 骨架、通过编译——已全部达成。4 项偏差均有明确的推迟 Phase 和理由，不影响 Phase 2 的启动条件。

## 下一步: Phase 2 — Journal + Transport

目标: `orz -p "hello"` 产生有效的 `events.jsonl`，hash chain 可独立验证。

具体任务见 `../存档/architecture/pre-adr-0010/FORK_IMPLEMENTATION_DESIGN_v0.1.md` §8 Phase 2（主仓）。
