# Phase 2 编译修复状态 v0.1

状态：2026-08-03。记录 `B:\orz` Phase 2（Journal + Transport）中 orz-shell 编译修复的实际进展、
阻断因素和分析。

## 1. 背景

Phase 1 将 Grok Build `500129c7` fork 为 orz：删除了 19 个 crate，将 21 个 crate 从 `xai-grok-*` 重命名为 `orz-*`。
批量重命名工具对 `.rs` 源文件也执行了 `xai_grok_*` → `orz_*` 的替换——**包括对已删除 crate 的引用**。

结果：310+ 个文件中对 14 个已删除 crate 的引用被"幽灵重命名"——代码 import `orz_telemetry`、`orz_sampler`、
`orz_chat_state` 等，但这些 crate 已不存在。

## 2. Phase 2 编译修复策略回顾

设计文档 `FORK_IMPLEMENTATION_DESIGN_v0.1.md` §8 的阶段定义：

| Phase | 目标 | 关键依赖 |
|-------|------|---------|
| Phase 1 | Scaffold + Build | orz-bin 可编译运行 ✅ |
| Phase 2 | Journal + Transport | **orz-shell 可编译** + orz-assurance journal |
| Phase 3 | Gates + Orientation | orz-shell 注入 |
| Phase 4 | TUI + Polish | 全部 |

Phase 2 的前提条件（orz-shell 可编译）在设计时未被充分估计——删除 19 个 crate 后，
orz-shell 的依赖图存在深层断裂，修复成本远超预期。

## 3. 编译修复实际进展

### 3.1 git 提交记录（`feat/phase1-scaffold` 分支）

```
64fd464 chore(phase1): close Phase 1 — dual binary scaffold verified
c736b1a feat(phase2): restore deleted crates + compatibility shims
<N>      chore(phase2): complete remaining compilation fixes
<N+1>    chore(phase2): restore shell-session-support crate + more stubs
```

### 3.2 恢复和创建的 crate

| Crate | 方式 | 状态 |
|-------|------|------|
| `orz-sampler` | 从 `500129c` 恢复 | ✅ 编译通过 |
| `orz-chat-state` | 从 `500129c` 恢复 | ✅ 编译通过 |
| `orz-auth` | 从 `500129c` 恢复 | ✅ 编译通过 |
| `orz-telemetry` | 手写 stub（21 模块） | ✅ 编译通过 |
| `orz-announcements` | 手写 stub | ✅ 编译通过 |
| `orz-plugin-marketplace` | 手写 stub | ⚠️ 不完整，需扩展 |
| `orz-subagent-resolution` | 手写 stub | ⚠️ 不完整，需扩展 |
| `orz-shell-session-support` | 从 `500129c` 恢复 | ⚠️ 编译通过但内部引用未完全修复 |

### 3.3 源文件级修复

| 修复项 | 文件数 | 说明 |
|--------|--------|------|
| `orz_shell_base` → 内联模块 | 3 | `cpu_profile`, `env`, `util` 模块并入 orz-shell |
| `xai_sqlite_journal::JournalMode` → 本地 stub | 7 | orz-workspace (2), orz-shell (4), orz-memory (1) |
| `xai_codebase_graph` → `codebase_graph_stub` | 5 | orz-workspace 内本地 stub |
| `xai_chat_state` → `orz_chat_state` | ~20 | orz-shell 内 bulk rename 遗漏修复 |
| `xai_grok_env` → `orz_env` | 1 | `env.rs` 引用修复 |
| `xai_grok_config` → `orz_config` | 1 | `grok_home.rs` 引用修复 |
| `xai_grok_version` → `orz_version` | 1 | `changelog.rs` 引用修复 |
| `to_managed_name` / `inject_managed_headers` | 1 | `managed_mcp.rs` 函数缺失（通过恢复 session-support 修复） |

## 4. 当前错误状态

### 4.1 编译结果

```
$ cargo check -p orz-shell
error: could not compile `orz-shell` (lib) due to 981 previous errors
```

所有 981 个错误均位于 `orz-shell` 自身。依赖 crate 全部编译通过。

### 4.2 错误分类

| 类别 | 数量（估计） | 根因 |
|------|-------------|------|
| `orz_telemetry::*` 缺失符号 | ~400 | stub 未覆盖完整的 telemetry API surface |
| `orz_plugin_marketplace::*` 缺失 | ~100 | stub 不完整 |
| `orz_subagent_resolution::*` 缺失 | ~50 | stub 不完整 |
| 类型不匹配（stub 与实际 API 差异） | ~200 | 手写 stub 的方法签名与实际调用不匹配 |
| 方法缺失 | ~150 | stub 缺少方法 |
| 其他 | ~80 | trait 实现缺失、泛型约束等 |

### 4.3 缺失的 orz_telemetry 符号（部分）

```
session_metrics:  DoomLoopRecovery, SessionStarted, TraceUploadAttempted,
                  TraceUploadFailed, TraceUploadSkipped, TraceUploadSucceeded,
                  TraceUploadReason, Turn, TurnCompletedLifecycle
events:           ManualAuthSurface, AuthTokenKind, ManualAuth, ManualAuthReason,
                  RolloutSurvey, ClientHookGateOutcome, CompactionRetryDegraded,
                  CompactionTrigger (~40+ more event structs still missing)
instrumentation:  ChromeTraceOptions, InstrumentationFinalizer, finalize,
                  finalizer, generate_chrome_trace
prompt_timing:    PromptTiming (~10+ types)
```

## 5. 问题分析

### 5.1 为什么 stub 策略不适用于此规模

1. **API surface 巨大**：`orz_telemetry` 原始 crate 有 21 个模块、~2200 行仅 `events.rs`。
   手写覆盖所有符号需要猜测 ~300+ 个函数签名、~80+ 个 struct 字段类型。

2. **签名必须精确**：Rust 的类型系统要求每个方法签名、泛型约束、trait 实现与调用点完全匹配。
   一个字段的类型错误（`u64` vs `usize`）会产生连锁编译错误。

3. **依赖深度**：orz-shell 直接或间接引用了 14 个已删除 crate 中的 ~12 个，
   每个都需要完整的（或至少足够的）API surface。

4. **迭代效率**：每轮 "编译 → 看错误 → 补 stub → 再编译" 需要 2-10 分钟（全仓编译时间长），
   且每次只暴露一层错误（Rust 在遇到一定数量错误后停止）。

### 5.2 设计文档与实现的差距

`FORK_IMPLEMENTATION_DESIGN_v0.1.md` §1.1 将 18 个 crate 标记为"删除"，但其中部分 crate
（如 `xai-grok-shell-session-support`）被 orz-shell 深度依赖。删除这些 crate 的决定
基于"功能不需要"的逻辑，但未充分评估"代码仍引用"的实际情况。

## 6. 推荐路径

基于以上分析，有三种可行方案（按推荐顺序）：

### 方案 A：从 git 批量恢复（推荐）

不创建手写 stub，而是从 git `500129c` 恢复 orz-shell 仍然需要的原始 crate，
做一次性批量重命名和依赖更新。

**需恢复的 crate**（估计）：
- `xai-grok-plugin-marketplace` → `orz-plugin-marketplace`（~15 文件）
- `xai-grok-subagent-resolution` → `orz-subagent-resolution`（~8 文件）
- `xai-grok-telemetry` → 替换手写 stub 为原始实现（~21 文件，已有部分 stub 可保留）

**预计迭代次数**：3-5 次
**预计总工作量**：2-4 小时

### 方案 B：不编译 orz-shell，使用独立 minimal agent

放弃编译完整的 orz-shell。Phase 2 的核心交付物是 `orz -p "hello"` → 有效的 `events.jsonl`。
可以直接使用 `orz-sampler` + `orz-sampling-types`（两者已编译）来构建最小 agent loop，
绕过 orz-shell 的编译问题。

**优点**：
- 不需要修复 orz-shell 的 981 个错误
- 可以快速进入 journal 系统实现
- agent loop 的实现更简单、可控

**缺点**：
- 偏离设计文档的"注入"模式
- 后续 Phase 3/4 仍需面对 orz-shell 编译问题
- 实际上是在构建一个新的 agent runtime

### 方案 C：保留 orz-shell 为 reference，分离编译范围

将 orz-shell 从 `cargo check` 目标中排除。Phase 2 仅实现：
1. `orz-assurance` journal 系统（独立编译，已完成骨架）
2. `orz-sampling-types` 中注入 `thinking: disabled`
3. 日记写入验证使用独立测试

orz-shell 的编译修复推迟到 Phase 3，届时采用方案 A（批量恢复）。

## 7. 当前分支状态

| 仓库 | 分支 | 最后提交 |
|------|------|---------|
| `B:\orz` | `feat/phase1-scaffold` | Phase 2 编译修复 WIP |
| `D:\CLI` | `main` | Phase 1 tech debt 关闭 |
| `D:\CLI` | `feat/fork-architecture` | 4 篇设计文档（v0.1-v0.3 + impl） |

## 8. 参考

- [`FORK_IMPLEMENTATION_DESIGN_v0.1.md`](FORK_IMPLEMENTATION_DESIGN_v0.1.md) — Phase 分步设计
- [`PHASE1_TECH_DEBT_v0.1.md`](PHASE1_TECH_DEBT_v0.1.md) — Phase 1 偏差和 §2.1 断裂统计
- `B:\orz\PHASE1_COMPLETION.md` — Phase 1 完成报告
