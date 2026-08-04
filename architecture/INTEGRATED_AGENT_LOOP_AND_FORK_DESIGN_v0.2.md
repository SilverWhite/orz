# 融合架构：Codex 纪律 + Grok 能力 v0.2

状态：2026-08-04 修正——放弃 Pro/Flash 双模型常驻（已暂时放弃实现这一构想，存档，非待办），
采用单主 Agent + 检索子代理 ×2（见 §4.5）。v0.1 整合了 Agent Loop Redesign 与 Fork Implementation Design，
解决了注入点分类、crate 矩阵和 Phase 规划问题。v0.2 基于对 `D:\CLI\orz` (74 crate, ~1.1M 行)
和 Codex CLI (~120 crate, core 151k 行) 的源码级审查，确认：**Grok 底座中仅 ~320k 行
（tools/workspace/sandbox/mcp/chat-state/hooks）是真正有价值的成熟组件。其余 ~780k 行
（shell + pager + telemetry + marketplace + announcements + sampler）是架构债。**

v0.2 将债务一次性清零。

### 参考

- v0.1 整合设计：`INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.1.md`（已被本文取代）
- 融合评估详情：参见本次会话中的三轮 agent 审查 + Codex/Grok 客观对比

---

## 0. 核心决策

```
v0.1 (保留 shell):                   v0.2 (融合架构):
  Grok 底座 ~1.1M 行                   Grok 底座 ~320k 行（仅提供者）
  ├── orz-shell (364k) ← 待手术        ├── orz-tools (127k, 2,769 tests)
  ├── Grok TUI (477k) ← 兜底          ├── orz-workspace (89k)
  ├── 提供者 (~230k) ← 有价值          ├── orz-sandbox (6k, Job Object)
  ├── 死代码 (~35k) ← 待消除           ├── orz-mcp (11k)
  └── 3 次未来手术                      ├── orz-chat-state + compaction (21k)
                                        ├── orz-hooks (9k)
  自研核心 (~46k)                       ├── orz-auth/secrets (1k)
  ├── orz-loop                          ├── orz-config/models/memory/markdown (42k)
  ├── orz-assurance                     └── xai-* 工具库 (~80k)
  ├── orz-tui
  └── orz-bin                           自研核心 (~20k)
                                        ├── **orz-host** (NEW ~8k) ← ACP + LoopHost
                                        ├── orz-loop (~2k)
                                        ├── orz-assurance (4-7k) ← 热路径 gate/journal/trigger
                                        ├── orz-tui (5-8k)
                                        └── orz-bin (1k)

                                        注: assurance Python 95k 行中仅 4-7k 热路径逻辑需 Rust 移植
                                        其余保留为 conformance suite + schema authority + 离线验证

继承代码: ~1.1M → ~320k (减 71%)
Crate 数: 74 → ~42 (减 43%)
未来手术: 3 → 0
```

### 为什么是融合

| 维度 | 取谁 | 具体内容 |
|------|------|---------|
| **架构纪律** | Codex | 薄核心、单向依赖、严格 Clippy、零死代码 crate |
| **ACP 协议** | Grok | `agent-client-protocol` 0.10.4 + `xai-acp-lib` |
| **Job Object 沙箱** | Grok | `orz-sandbox` (6,355 行，生产验证) |
| **工具实现** | Grok | `orz-tools` (126,694 行, 2,769 tests) |
| **FS/VCS/权限** | Grok | `orz-workspace` (89,370 行, 含 permission manager) |
| **持久化/Compaction** | Grok | `orz-chat-state` + `orz-compaction` |
| **Hook 系统** | Grok | `orz-hooks` |
| **MCP** | Grok | `orz-mcp` |
| **自研核心** | orz | orz-loop (Agent Loop) + orz-assurance (gates/journal/snapshot) + orz-tui |

### 删除清单（一次性，不留手术）

| Crate | 行数 | 类别 |
|-------|------|------|
| orz-shell | 364,643 | 耦合根源 → 被 orz-host 替代 |
| orz-shell-session-support | 1,352 | 随 shell |
| xai-grok-pager | 462,037 | Grok TUI → 被 orz-tui 替代 |
| xai-grok-pager-render | 34,719 | 随 pager |
| xai-grok-pager-minimal | 6,256 | 随 pager |
| xai-grok-pager-bin | 3,399 | Grok 兜底入口 |
| xai-ratatui-inline | 3,290 | 仅 pager 使用 |
| xai-prompt-queue | 440 | 仅 shell+pager 使用 |
| orz-telemetry | 14,960 | 死代码 |
| orz-plugin-marketplace | 5,622 | 死代码 |
| orz-announcements | 443 | 死代码 |
| orz-sampler | 11,373 | 被 orz-loop::gateway::transport 替代 |
| orz-http | 686 | 薄 wrapper → orz-loop 直接用 reqwest |
| orz-config-types | 3,562 | 评估后删除 |

---

## 1. 架构全景

```
┌──────────────────────────────────────────────────────────────────┐
│                         orz 二进制 (orz-bin)                       │
│                                                                   │
│  orz-tui (assurance workbench)  ←──── ACP JSON-RPC ────→        │
│                                                         │         │
│  ┌──────────────────────────────────────────────────────┼───────┐ │
│  │               orz-host (NEW — ~8k 行)                 │       │ │
│  │                                                       │       │ │
│  │  ACP Server (agent-client-protocol 0.10.4 + acp-lib) │       │ │
│  │  Session Bootstrap (trust, envelope, journal create)  │       │ │
│  │                                                       │       │ │
│  │  impl LoopHost for OrzHost:                           │       │ │
│  │    tools_registry()     → orz-tools-api               │       │ │
│  │    call_tool()          → orz-tools                    │       │ │
│  │    request_permission() → orz-workspace::permission    │       │ │
│  │    persist_turn()       → orz-chat-state               │       │ │
│  │    trigger_compaction() → orz-compaction               │       │ │
│  │    run_hooks()          → orz-hooks                    │       │ │
│  │    auth_credentials()   → orz-auth / orz-secrets       │       │ │
│  │    mcp_tools()          → orz-mcp                      │       │ │
│  │                                                       │       │ │
│  │  Journal Writer (hash-chained JSONL, 阻塞 send)       │       │ │
│  │  Interactive Approval Prompter (ACP ApprovalRequest)  │       │ │
│  │  Telemetry: 硬无 (编译时 gate, 无初始化)               │       │ │
│  └──────────────────────┬───────────────────────────────┘       │
│                         │                                        │
│  ┌──────────────────────┼───────────────────────────────┐       │
│  │                orz-loop (自研 — ~2k 行)               │       │
│  │                                                       │       │
│  │  AgentLoopController                                   │       │
│  │    ├─ PromptBuilder      (TOOL_AVAIL/CHECKPOINT/SUFFICIENCY) │
│  │    ├─ ModelGateway       (thinking:disabled + transport/)    │
│  │    ├─ ToolDispatcher     (IPG + snapshot + counters)  │       │
│  │    ├─ OrientationMonitor (事件驱动 + cooldown)         │       │
│  │    ├─ MechanicalRelay    (纯 function.name 路由)       │       │
│  │    ├─ Blackboard         (5 分区)                      │       │
│  │    ├─ 主 Agent            (DeepSeek V4)                │       │
│  │    └─ RetrievalSubagents (内部 + 外部)                 │       │
│  └──────────────────────┬───────────────────────────────┘       │
│                         │                                        │
│  ┌──────────────────────┼───────────────────────────────┐       │
│  │              orz-assurance (自研 — 4-7k 行热路径)       │       │
│  │  gates/  orientation/  journal/  session/            │       │
│  │  sandbox/  credential/  permit/                      │       │
│  └──────────────────────┬───────────────────────────────┘       │
│                         │                                        │
│  ┌──────────────────────┼───────────────────────────────┐       │
│  │        Grok 成熟底座 (保留 — ~320k 行)                │       │
│  │  orz-tools  orz-workspace  orz-sandbox  orz-mcp      │       │
│  │  orz-chat-state  orz-compaction  orz-hooks            │       │
│  │  orz-auth  orz-secrets  orz-config  orz-memory       │       │
│  │  orz-markdown  xai-acp-lib  xai-* 工具 crates        │       │
│  └──────────────────────────────────────────────────────┘       │
└──────────────────────────────────────────────────────────────────┘

依赖方向（强制单向，Codex 纪律）：
  orz-bin → orz-tui → orz-host → orz-loop → orz-assurance
                    ↘ orz-host → {Grok 底座}  (LoopHost trait 解耦)
  Grok 底座 crate 永不反向依赖任何自研 crate
```

---

## 2. Crate 矩阵

### 2.1 保留（~320k 行，不改代码）

| Crate | 行数 | 功能 |
|-------|------|------|
| orz-tools | 126,694 | 工具实现 (terminal/file/edit/search/bash/skills/LSP)，2,769 tests |
| orz-tools-api | 829 | 工具接口类型 |
| orz-workspace | 89,370 | FS/VCS/execution/permission pipeline (含 IP6 执行点) |
| orz-workspace-client | 824 | workspace 轻量客户端 |
| orz-workspace-types | 8,548 | workspace 类型定义 |
| orz-sandbox | 6,355 | 跨平台 sandbox (Job Object/Landlock/Seatbelt) |
| orz-mcp | 10,610 | MCP 协议 (rmcp 2.1) + OAuth/credentials/servers |
| orz-chat-state | 14,137 | 对话持久化 (Actor-based) |
| orz-compaction | (common/) | 上下文压缩 |
| orz-hooks | 9,246 | hook 系统 (PreToolUse/PostToolUse/SessionStart/Stop) |
| orz-auth | 411 | 凭据管理 (token provider + retry middleware) |
| orz-secrets | 568 | 凭据脱敏 |
| orz-memory | 9,935 | 记忆系统 (默认关闭) |
| orz-config | 11,155 | 配置解析 |
| orz-models | 70 | 模型定义 |
| orz-markdown | 20,226 | 流式终端 markdown 渲染 |
| orz-markdown-core | 993 | markdown 渲染核心 |
| xai-acp-lib | 2,277 | ACP gateway/channels (over agent-client-protocol 0.10.4) |
| xai-agent-lifecycle | 515 | agent 生命周期 |
| xai-file-utils | 14,564 | 文件工具 |
| xai-fast-worktree | 20,161 | 快速 git worktree |
| xai-fsnotify | 6,016 | 文件系统通知 |
| xai-gix-status | 553 | git status |
| xai-hooks-plugins-types | 1,140 | hook 插件类型 |
| xai-token-estimation | 228 | token 估算 |
| xai-tty-utils | 1,523 | TTY 工具 |
| xai-system-power | 668 | 系统电源 |
| xai-tracing-macros | 202 | 诊断宏 |
| xai-crash-handler | 1,878 | 崩溃处理 |
| xai-workflow | 3,089 | 工作流 |
| xai-hunk-tracker | 11,352 | 变更追踪 |
| ptyctl / ptyctl-cli | 2,840 | PTY 控制 |
| xai-ratatui-textarea | 13,037 | TUI 文本区组件 (orz-markdown 使用) |
| orz-paths | 491 | 路径工具 |
| orz-env | 197 | 环境变量 |
| orz-shared | 4,979 | 共享工具 |
| orz-version | 69 | 版本 |
| orz-agent | 20,912 | agent 基础类型 |

### 2.2 删除（一次完成）

| Crate | 理由 |
|-------|------|
| orz-shell (364,643) | 被 orz-host 替代 |
| orz-shell-session-support (1,352) | 随 shell |
| xai-grok-pager (462,037) | 被 orz-tui 替代 |
| xai-grok-pager-render (34,719) | 随 pager |
| xai-grok-pager-minimal (6,256) | 随 pager |
| xai-grok-pager-bin (3,399) | Grok 兜底入口 |
| xai-ratatui-inline (3,290) | 仅 pager 使用 |
| xai-prompt-queue (440) | 仅 shell+pager 使用 |
| orz-telemetry (14,960) | 不外发遥测 |
| orz-plugin-marketplace (5,622) | 不需要 |
| orz-announcements (443) | 不需要 |
| orz-sampler (11,373) | transport 移入 orz-loop，actor 删除 |
| orz-http (686) | 薄 wrapper，orz-loop 直接用 reqwest |
| orz-config-types (3,562) | 合并入 orz-config 或删除 |

### 2.3 新增

| Crate | 行数(估) | 内容 |
|-------|---------|------|
| **orz-host** | 6-12k | ACP server + LoopHost impl + session lifecycle + approval prompter |
| orz-bin | ~1k | composition root |
| orz-loop | ~2k | AgentLoopController + Blackboard + MechanicalRelay + 主 Agent + Subagents |
| orz-loop::gateway::transport | ~7.6k | 从 orz-sampler 移入的 SSE streaming + retry |
| orz-assurance | 4-7k | 热路径: journal+gate+orientation+trust。其余 Python 95k 保留为 conformance suite |
| orz-tui | 5-8k | assurance workbench TUI |

### 2.4 双 TUI 策略

- **主 UI**: `orz-tui` (自研 assurance workbench, ACP 通信)
- **兜底 UI**: Codex ratatui TUI (~20k 行轻量前端)。orz-host 同时暴露 ACP (给 orz-tui) 和 Codex app-server JSON-RPC (给兜底)。Codex TUI 不展示 assurance 面板，但 assurance 层在 orz-host 中完整静默运行

---

## 3. orz-host 设计

### 3.1 定位

orz-host 是融合架构的关键——**一个 ~8k 行的薄核心，替代 orz-shell 的 364k 行**。
它不包含 agent loop（在 orz-loop 中），不包含 assurance 逻辑（在 orz-assurance 中），
只做一件事：**把 Grok 提供者 crate 桥接到自研核心**。

### 3.2 依赖

```toml
[package]
name = "orz-host"
version = "0.1.0"
edition = "2024"

[dependencies]
# ACP protocol
agent-client-protocol = "0.10"
xai-acp-lib = { path = "../codegen/xai-acp-lib" }

# Self-built core (单向依赖)
orz-loop = { path = "../orz-loop" }
orz-assurance = { path = "../orz-assurance" }

# Grok providers (only public APIs)
orz-tools = { path = "../codegen/orz-tools" }
orz-tools-api = { path = "../codegen/orz-tools-api" }
orz-workspace = { path = "../codegen/orz-workspace" }
orz-sandbox = { path = "../codegen/orz-sandbox" }
orz-mcp = { path = "../codegen/orz-mcp" }
orz-chat-state = { path = "../codegen/orz-chat-state" }
orz-hooks = { path = "../codegen/orz-hooks" }
orz-auth = { path = "../codegen/orz-auth" }
orz-secrets = { path = "../codegen/orz-secrets" }
orz-config = { path = "../codegen/orz-config" }
orz-models = { path = "../codegen/orz-models" }

tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
```

### 3.3 结构

```
crates/orz-host/
├── Cargo.toml
└── src/
    ├── lib.rs              # OrzHost struct + LoopHost impl
    ├── acp_server.rs       # ACP JSON-RPC stdio server (session/new, session/prompt, ...)
    ├── session.rs          # Session bootstrap (trust, envelope, journal create)
    ├── approval.rs         # Interactive approval prompter (ACP ApprovalRequest channel)
    └── config.rs           # Host configuration wiring
```

### 3.4 LoopHost trait（在 orz-loop 中定义，orz-host 实现）

```rust
// orz-loop/src/host.rs

#[async_trait]
pub trait LoopHost: Send + Sync {
    fn tools_registry(&self) -> &dyn ToolRegistry;
    async fn call_tool(&self, name: &str, args: Value, call_id: &str)
        -> Result<ToolResult, ToolError>;
    async fn request_permission(&self, risk: RiskClass, tool: &str, args: &Value)
        -> Result<PermitDecision, PermitError>;
    async fn persist_turn(&self, turn: &TurnRecord) -> Result<(), PersistError>;
    async fn trigger_compaction(&self) -> Result<(), CompactionError>;
    async fn run_hooks(&self, event: &HookEvent) -> Vec<HookResult>;
    fn auth_credentials(&self) -> Result<Credentials, AuthError>;
    fn mcp_tools(&self) -> Vec<ToolDef>;
    fn journal(&self) -> &JournalRecorder;
}
```

---

## 4. Assurance 功能分类（继承自 v0.1，简化）

由于 orz-shell 被删除，IP4a-c 不再是"注入 kept 组件"——它们在 orz-host 中原生实现。
IP6 仍是唯一需要修改 kept Grok 组件的 assurance 功能。

| 功能 | 实现位置 | 类型 |
|------|---------|------|
| IP1 thinking:disabled | orz-loop::ModelGateway | 原生 |
| IP2a TOOL_AVAILABILITY | orz-loop::PromptBuilder | 原生 |
| IP2b ORIENTATION_CHECKPOINT | orz-loop Agent Loop 主循环 | 原生 |
| IP2c INFO_SUFFICIENCY | orz-loop::PromptBuilder | 原生 |
| IP3a IPG gate | orz-loop::ToolDispatcher (wrapper) | 原生 |
| IP3b orientation counters | orz-loop::ToolDispatcher (wrapper) | 原生 |
| IP3c sufficiency trigger | orz-loop::ToolDispatcher (wrapper) | 原生 |
| IP4a session lifecycle | orz-host::session | 原生 (新) |
| IP4b turn lifecycle | orz-host ACP prompt handler | 原生 (新) |
| IP4c session close | orz-host ACP session close | 原生 (新) |
| IP5 pre-mutation snapshot | orz-loop::ToolDispatcher (wrapper) | 原生 |
| IP6 hard-gate enforcement | **orz-workspace::permission (注入)** | **唯一的注入点** |

---

## 4.5 2026-08-04 架构修正：单主 Agent 裁决

**裁决**：放弃 Pro/Flash 双模型常驻构想，采用**单主 Agent + 检索子代理 ×2**。
Pro/Flash 概念保留于此文档作为存档，标注「**已暂时放弃实现这一构想**」——非待办项，不作为后续默认计划。

依据：
1. Two-Subagent Default Constraint（`CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30`）：不得默认发展本地多代理调度器；双模型常驻即本地多代理调度器。
2. 规划/执行分离由 plan mode 状态机承载（Python R15 已验证交付），非第二模型实例；Codex 纪律的本义是模式切换（同一模型的 prompt/权限切换），不是双模型常驻。
3. Python 侧全部先例（canonical loop、orientation_runtime_journal）均为单 agent 语义，conformance 可直接对齐。
4. 单模型入口使 IPG/permission 门控语义简单；Blackboard 无跨 agent 格式契约（主 agent 自写自读）。
5. 上下文长度由用户控制（常新开对话）；单轮任务内双模型上下文拆分收益有限。

连带调整：§1 架构图、§2.3 crate 矩阵、§5 Phase 2 的 Pro/Flash 行均改为「主 Agent（DeepSeek V4）」。
若未来出现长会话多模型调度场景，重新评估此构想（届时以新设计版本/ADR 处理）。

---

## 4.6 中立问询与反例询问触发设计（2026-08-04 定稿）

承接 Neutral vs Counterexample Trigger Split（2026-07-30，CN 设计约束）与子代理检索完成确认（CN §7.3），将 IP2c/IP3b/IP3c 的触发语义定稿。原则：**中立问询留在执行过程中，收尾只留反例询问；过犹不及**。

### 4.6.1 触发位置总表

| 位置 | 机制 | 触发规则 |
|---|---|---|
| 每轮检索动作彻底完成后 | 中立问询（信息充分性：IP2c block + IP3c trigger） | 4 判定点任一 → 同一问询 → **触发瞬间 4 计数全部清零** |
| 关闭检索工具/检索子代理前 | 中立 completion check | 一次性（Python 先例 `RETRIEVAL_COMPLETION_CHECK v0.1`，Rust 接线） |
| plan 写入前 | 反例询问 | 保留（plan approval gate 链组成部分） |
| 正式答案输出前 | 反例询问 | **仅触发一次 + message_block 显式告知模型** |

### 4.6.2 裁决

1. **正式答案输出前不触发中立问询**（过犹不及）：模型能力足够，多加只是负担；且输出前中立问询的补救窗口（"还能补检索"）已关闭，价值近零。
2. **正式答案输出前的反例询问仅触发一次**，message_block 显式告知"本询问仅出现一次"——与轮内问询区分收尾 gate 语义，让模型知道这是最后一次自查机会。
3. **plan 写入前的反例询问保留**——它是 plan approval gate 链的组成部分，位置不同（plan 阶段模型无"即将输出"紧迫感），不在"仅一次"范围内。

### 4.6.3 4 判定点与清零状态机（IP3b，轮内中立问询）

- 4 判定点（满足任一即触发同一中立问询）：**输出阈值**（stagnation consecutive/ngram repeat > 阈值）、**工具调用次数**、**动作次数**、**轮次**。
- **触发即清零**：问询触发瞬间 4 个计数全部归零并重新积累（非模型响应后）。
  - 同作用问询 = 同一事件源；不共享重置点会导致未触发计数器在下一轮立即补触发（问询刷屏）。
  - 清零形成**隐式冷却**：任何问询后必须重新积累阈值单位才可能再次触发。
  - **取代 CN 约束的轮次豁免（2026-08-04 裁决）**：CN `GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN` 规定"轮次触发不受 cooldown 限制"；本定稿对**轮次计数同样清零**——4 判定点同作用共享重置点，用户批准的全清零方案优先，跨轮漂移由轮内其他 3 个判定点兜底。
  - stagnation 的 `RestartRequested`（terminal_safe_restart_packet）语义为全局重启，本身重置一切，与清零不冲突。
- **作用域**：仅作用于轮内中立问询；子代理关闭前 completion check 为独立一次性收尾检查，无积累语义，**不参与 4 计数**。
- **默认阈值（Phase 3 落地时定稿，参照 CN 门控默认）**：输出阈值沿用 stagnation 默认（consecutive/ngram 严格大于 10）；工具调用次数 / 动作次数 / 轮次的默认阈值参照 CN 约束（10 动作 / 8 轮）——Phase 3 接线时按 §4.6.3 语义定稿，数值调整以 ADR 记录。
- **2026-08-04 接线定稿（ADR-0005）**：4 判定点全部严格大于——输出阈值 > 10（consecutive/ngram，沿用 stagnation 默认）、工具调用次数 > 10、动作次数 > 10、轮次 > 8；触发检查顺序固定为 output_repeats → tool_calls → actions → rounds（返回第一个命中）；触发瞬间**三实例（主 agent + 检索子代理 ×2）4 计数全部清零**。数值后续调整以新 ADR 记录。

### 4.6.4 子代理动作语义分层（动作次数判定）

- 主 agent 与检索子代理除职责、prompt、权限外配置**一致且各自独立**（承接 Two-Subagent Default Constraint 精神——约束原文只规定子代理数量与类型，配置一致性是本定稿的细化）。
- **动作次数计数粒度角色感知**——同一计数器接口，不同事件源：
  - 主 agent：底层工具级事件（`ToolDispatcher` 包装）。
  - 子代理：**上层语义动作**——一次 `run_retrieval`（一个 goal 执行）= 1 个动作；`budget_turns > 1` 时每轮 = 1 个动作。子代理内部**关闭工具级计数**（检索本身动作多——search/fetch/read 多工具调用，工具级计数必然误触发）。
- 其余 3 判定点（输出阈值/工具调用/轮次）主 agent 与子代理照常。

### 4.6.5 Block 文案模板（格式对齐 Python `[BLOCK v0.1]` 惯例）

反例询问（正式答案输出前，仅一次）：

```text
[COUNTEREXAMPLE_GATE v0.1]
最终回答即将输出。请对即将输出的结论做最后一次反例自查：
1. 是否存在未验证的前提？
2. 是否存在可推翻结论的已知证据？
3. 结论强度是否超出证据支持？
注意：本反例询问仅出现一次，请在最终回答前完成全部反例自查。
[/COUNTEREXAMPLE_GATE]
```

中立问询（每轮检索动作彻底完成后，IP2c）：

```text
[INFO_SUFFICIENCY v0.1]
本轮检索已完成。请确认：
是否已获得完成当前主任务所需的内容？
请回答 yes / no / uncertain，并附简短理由。
若为 no 或 uncertain，只列出还需要的内容类型。
[/INFO_SUFFICIENCY]
```

### 4.6.6 实现位置（承接 §4 表格；2026-08-04 已接线）

- IP2c INFO_SUFFICIENCY block：`orz-loop::PromptBuilder`（原生）——常量位于 `orz-loop/src/prompt.rs`（`INFO_SUFFICIENCY_BLOCK`），以 `Role::User` message 注入（对模型的运行时提问而非系统上下文）
- IP3b 4 判定点计数器 + 清零状态机：`orz-loop` 控制器持有状态（触发即清零）；计数事件源——主 agent 吃工具级（`ToolDispatcher` 包装），子代理吃语义级（`run_retrieval` / 轮）——`orz-loop/src/inquiry.rs`（`InquiryCounters`/`InquiryThresholds`/`DEFAULT_THRESHOLDS`）
- IP3c sufficiency trigger：每轮检索动作彻底完成后检查（`ToolDispatcher` 包装后置）——`controller.rs` 工具循环内检索分发后触发；三实例同清零
- 子代理关闭前 completion check：子代理关闭路径接线 Python 先例 `RETRIEVAL_COMPLETION_CHECK v0.1`（Rust 侧移植）——block 注入子代理请求（`agents/retrieval.rs` `completion_check_block` 参数），响应逐行解析 yes/no/uncertain（`parse_completion_decision`），事件带完整响应文本作证据
- 反例询问：plan 写入前 + 正式答案输出前（仅一次 + 显式告知）——`run_plan` 在 `submit_plan` 前做一次模型轮（plan 变体 block 无"仅一次"行；机械 plan 下为 evidence-only，模型生成 plan 时升级为阻塞链环节）；正式答案输出前在工具循环无 tool_calls 分支拦截一次（`COUNTEREXAMPLE_GATE_BLOCK`，被拦截草稿 journal 为 model_output 但不进入会话）
- 三个 gate 均记录 run-event：`neutral_inquiry` / `counterexample_gate` / `retrieval_completion_check`（run-event schema 27→30 变体，payload schema 见 `runtime/`）；模型回答无控制流后果（证据记录，循环自然继续）

---

## 5. 实施阶段

### Phase 0: 融合删除（本次）
**目标**：`D:\CLI\orz` workspace 仅保留提供者 + orz-host skeleton，`cargo check` 绿色

1. 创建分支 `feat/fusion-architecture`
2. `git rm -r` 删除清单中的所有 crate
3. 更新 workspace `Cargo.toml` members + 移除 `[[bin]]` grok 声明
4. 创建 `crates/orz-host/` skeleton（空 ACP server + LoopHost stub）
5. 修复编译断裂（预期低——仅 pager 和 prompt-queue 引用 shell）
6. **验证**: `cargo check --workspace` 绿色，成员数由 74 → ~42

### Phase 1: Journal + Transport + 单 Agent Loop
**目标**：`orz -p "hello"` → 有效 `events.jsonl`

1. orz-assurance::journal (event, chain, recorder, verifier)
2. orz-loop skeleton + ModelGateway (含 transport/) + 单 Agent loop
3. orz-host: ACP server 完整实现 + session bootstrap + journal writer
4. Wire the seam: ACP prompt → AgentLoopController.run_turn()
5. **验证**: `orz -p "hello"` → 有效 events.jsonl, hash chain 连续

### Phase 2: 单主 Agent + Blackboard + Gates
**目标**：单主 Agent + 检索子代理 + 完整 gate 链（注：Pro/Flash 双模型常驻已暂时放弃实现，见 §4.5）

1. Blackboard + MechanicalRelay + 主 Agent + Subagents
2. orz-assurance::gates + orientation
3. IP6: hard-gate @ orz-workspace permission
4. **验证**: 完整 gate 链

### Phase 3: TUI + Snapshot + Polish
**目标**：完整产品

1. orz-tui → assurance workbench
2. orz-assurance::session::snapshot
3. orz-assurance::sandbox::job_object, credential, permit
4. Codex 纪律：严格 Clippy，零死代码 crate
5. Python 项目 → reference-spec
6. **§4.6 接线**（2026-08-04 审查补列；2026-08-04 已接线，部分闭合）：反例询问（plan 写入前 + 正式答案输出前**仅一次 + message_block 显式告知**）——`run_plan` submit_plan 前模型轮（plan 变体无"仅一次"行）+ 工具循环无 tool_calls 分支拦截一次；中立问询接线（IP2c `INFO_SUFFICIENCY` block + IP3c 每轮检索后触发 + 4 判定点计数/清零状态机，阈值定稿见 §4.6.3/ADR-0005）；子代理关闭前 completion check（Python `RETRIEVAL_COMPLETION_CHECK` 移植，逐字）；三机制均记 run-event（新增 3 事件类型 + payload schema，Python authority 先行）。OrzHost + PermissionBridge 接入三入口已于 Slice #1 完成。已知边界：模型回答暂不驱动控制流（evidence-only）；plan 变体 gate 在机械 plan 下为证据性触发
7. **验证**: 全功能 conformance suite

---

## 6. 与 v0.1 的关键差异

| | v0.1 (Grok fork 继续) | v0.2 (融合架构) |
|---|---|---|
| 继承代码 | ~1.1M 行 | **~320k 行** |
| Workspace members | 74 | **~42** |
| orz-shell | 保留 (364k, 待 3 次手术) | **删除** |
| Grok TUI 兜底 | 保留 (477k) | **删除** |
| ACP 入口 | 注入 orz-shell acp_agent.rs | **orz-host 原生** |
| 未来手术 | 3 次 (turn/telemetry/pager) | **0** |
| 架构纪律 | Grok 原生 | **Codex 式** |
| Grok 提供者 | 保留 | 保留 (完全相同) |
| ACP | 原生 | 原生 (agent-client-protocol + xai-acp-lib) |
| Job Object | 原生 | 原生 (orz-sandbox) |
| 工具测试 | 2,769 | 2,769 (不变，keept crate 未修改) |
