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

### Windows-first 声明与证据边界（2026-08-09 补充）

- ORZ 当前定位为 **Windows-first / Windows-only pre-beta**，不因已能在维护者环境运行就宣称完成系统性 Windows 原生适配。
- 基础 Windows 可运行性中，Grok Build/Rust 生态的 inherited 能力与 ORZ added/modified 的专项加固必须分开记录；事故、修复和兼容性结论不得混为一谈。
- 后续适配采用“追加式事故台账 -> 脱敏精选案例库 -> 机械回归测试 -> beta 环境证据”闭环。任务跑分衡量 agent 解题能力，案例闭环率/环境覆盖/复发率衡量 Windows 工程成熟度，二者不得互相替代。
- 详细观察面、案例字段、晋级门槛与 beta 要求以 `WINDOWS_RUNTIME_CONTRACT_v0.1.md` §6 为准。

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

> **IP5 接线裁决（2026-08-05 Slice #4）**：变异类工具（`modifies_files`，bash 等不可静态得知目标的除外）在权限放行后、`tool_started` 前对目标路径 `track` 并记 `snapshot_created` 事件（`snapshot_hash` / `snapshot_error` 二选一）。快照是 **evidence 层非 gate**——track 失败 journal 记录、不阻断工具（偏离 v0.1 §3.5 注入代码的 `?` 传播，理由：证据失败不应使合法工具不可用，且失败本身被记录）。恢复路径（v0.1 §3.5 "用户显式请求时 restore，记录到 journal"）**尚未接线**——`SnapshotStore::restore/revert` 模块可用，host 无恢复入口（approval/TUI 落地时补）。
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

## 4.6 问询机制触发设计（2026-08-04 定稿；2026-08-09 修订——恢复两机制分离）

承接 Neutral vs Counterexample Trigger Split（2026-07-30，CN 设计约束）、原始设计 `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`（修正 2/3）与子代理检索完成确认（CN §7.3），将各问询机制的触发语义定稿。原则：**机械化门禁不随便扩大范围；收尾只留反例询问；过犹不及**。

### 4.6.0 2026-08-09 修订记录（TB hard B 组复盘驱动）

2026-08-08/09 Terminal-Bench 探索性诊断（4 FAIL 方向全对但"深入单一方向"全程零拦阻）驱动设计复查，发现 **2026-08-04 定稿的合并错误**：

- 定稿将原始设计（07-26）两个独立机制错误合并——**方向问询**（ORIENTATION_CHECKPOINT，询问"当前任务内容与进度"）的 4 判定点被改绑到**信息充分性问询**（INFO_SUFFICIENCY）上；**方向问询被吞并**。
- 实现进一步退化：`controller.rs` orientation_checkpoint 仅剩事件记录（每轮一次，message_block 只进事件**不进模型消息**）——方向问询从未注入模型。现行"中立问询"实为**畸形拼合**：方向问询的骨架（4 判定点 + 动作间隙检查）× 信息充分性的文案（INFO_SUFFICIENCY）——TB 场景（无检索轮）高频误触发 + 文本-场景错位；模型无视（习惯性忽视）反而是正确行为。
- 本次修订**恢复原始设计的两机制分离**，完整保留各机制设计要素（不窄化触发时机、不吞并机制、不丢门控）。修订内容：§4.6.1 总表、§4.6.2 方向问询（判定点 + tool_variety 回归后删——2026-08-09 用户裁决）、§4.6.3 信息充分性（与判定点解绑 + 原始机械事实文案 + **子代理生命周期实现偏差记录**）、§4.6.5 反例询问职责边界明示、§4.6.8 实现位置。
- **2026-08-09 第三次修订（用户裁决）**：**tool_variety 彻底去掉**（原始语义与实测失效模式不匹配——TB 为"单一方向深入"非"发散"；高难度任务多工具是常态；轮次兜底）+ **轮次判定 8→7**（轮次承担兜底职责，最坏 7 轮一次方向问询）；token 门控注记明确不再并入（同理由：低估模型能力，少限制）。
- **2026-08-09 第二次修订（用户指示）**：① 信息充分性问询**严格恢复原设计**（`INFO_SUFFICIENCY_CHECK` 名称 + 机械事实模板全文，无任何简化——定稿简化版"本轮检索已完成"彻底废弃）；② **子代理生命周期实现偏差记录**（§4.6.3 注记）：设计 = 单轮检索任务内**持久化 + 显式开关**（主代理打开=派遣合同 / 关闭=确认本轮完毕，关闭不清零可重新唤醒，关闭回执 can_resume——`docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md` 原文），子代理动作面 = **调用本地浏览器**（外部检索，LBR-001 默认主路径）+ 项目文档（内部检索），结构化结果（query_summary / source_ledger / filtering_log / organized_response / raw_source_refs）——机械事实字段（来源数/覆盖/全文可见/缺失）的**数据出处正是 source_ledger 与 query_summary**；实现 = 瞬态单轮调用 + 零工具子代理（`agents/retrieval.rs` `tools: Vec::new()`——无法调用浏览器/任何工具）+ 自由文本结果（[DOC]/[来源] 行解析）——**实现偏差**。
- **案例化（2026-08-09，用户指示"值得作为案例来优化 orz"）**：本轮暴露两类可防问题——**设计退化**（08-04 定稿吞并方向问询、丢 tool_variety/token 门控、简化信息充分性文案）与**实现偏差**（方向问询仅事件记录无模型交互；子代理瞬态化零工具化）。教训：① 定稿合并机制/修改文案必须显式评审并保留原始设计要素（不吞并不窄化不简化）；② 实现接线必须回查设计原文（本案例：`RETRIEVAL_SUBAGENT_AUDIT` 生命周期原文 vs 瞬态实现）；③ 机械化门禁不扩大范围；④ 退化修复 = 先恢复原设计全文、再谈增强。落点：本设计文档修订记录 + memory；正式案例库形态待用户确认（可参照 WINDOWS_RUNTIME_CONTRACT §6 案例库结构）。

### 4.6.1 机制总览（2026-08-09 修订版）

| 机制 | 触发时机 | 门控/绑定 | 内容 | 回答后果 |
|---|---|---|---|---|
| 方向问询（ORIENTATION_CHECKPOINT） | 动作间隙（每工具轮后检查） | **4 判定点**：输出重复 >10 / 工具调用 >10 / 动作 >10 / 轮次 >7（2026-08-09 起，tool_variety 已删）——任一超限触发同一方向问询，触发瞬间全部清零（隐式冷却） | 当前正在做什么 / 任务定位 / 下一步服务哪个用户目标 | 无控制流后果（中性定位标记，证据记录） |
| 信息充分性（INFO_SUFFICIENCY_CHECK） | 检索动作彻底完成后 | `info_sufficiency_after_retrieval` 独立开关——**不绑 5 判定点** | 机械可验证事实（来源数/覆盖/全文可见/缺失类别）+ 是否足以完成任务 | 无控制流后果（证据记录；治本方向见 §4.6.3 注记） |
| 子代理 completion check（RETRIEVAL_COMPLETION_CHECK） | 关闭检索子代理前 | 一次性，独立 | 是否已获得完成主任务所需内容（yes/no/uncertain） | 无控制流后果（中性——claim_policy 全 False） |
| 反例询问（COUNTEREXAMPLE_GATE） | plan 写入前 + 正式答案输出前 | 正式答案变体**仅一次 + 显式告知** | 反例自查（前提/反证/结论强度） | 无控制流后果（自查引导） |
| 停滞守卫（runtime_stagnation_guard） | 公开输出连续/ngram 重复 > 阈值 | 独立机械门禁 | restart_requested / continue / handoff | **有**（restart = 全局重启） |

### 4.6.2 方向问询（原始设计恢复——询问任务内容与进度；机制最复杂，绑定 5 判定点）

- **职责**：执行过程中的**全局回看与方向辅助**——中性定位标记，维护主 agent 对"当前正在做什么 / 当前任务定位 / 下一步服务哪个用户目标"的清醒认知；防"深入单一方向"与"单轮异常发散"。
- **触发时机**：动作间隙——每工具轮后检查（原始设计修正 3 事件驱动语义：`action_completed ← 触发点`）；跨轮漂移（每轮都短、都正常但累计已偏）由**轮次判定点（>7 轮，2026-08-09 起）兜底**——清零语义下最坏 7 轮触发一次方向问询（注记见下；"固定间隔必触发"定时器语义未采纳）。
- **判定点（4，任一超限触发同一方向问询；2026-08-09 用户裁决——tool_variety 彻底去掉 + 轮次 8→7 兜底）**：
  - 输出重复 > 10（consecutive/ngram，沿用 stagnation 默认，按轮测量）
  - 工具调用 > 10
  - 动作 > 10
  - 轮次 > 7（**2026-08-09 由 >8 下调——轮次判定承担兜底职责**：其他判定点未抓住时，轮次最坏 7 轮触发一次方向问询）
  - 阈值与数值调整以新 ADR 记录。
  - **tool_variety 删除理由（2026-08-09 用户裁决）**：原始语义（单轮工具种类 >7 = 发散）防"单轮异常发散"，与实测失效模式不匹配（TB 4 FAIL 为"单一方向深入"——低多样性高数量，tool_variety 测不到）；高难度复杂任务单轮多工具是常态（GrokBuild 工具面 30+，7 种远低于正常使用面）——误报风险高于价值；发散/漂移检测由轮次判定兜底。原始 `orientation_trigger_tool_variety: 7` 至此不再恢复。
  - **token 门控注记更新**：原始 `orientation_trigger_tokens: 7000` 同样不再考虑并入（2026-08-09 同理由：低估模型能力，少限制；需轮内输出 token 度量链路）。
- **清零状态机（保留定稿有效部分）**：触发瞬间**全部计数清零**并重新积累（隐式冷却——任何问询后必须重新积累阈值单位才可能再次触发；同作用问询 = 同一事件源，不共享重置点会导致未触发计数器下一轮立即补触发即问询刷屏）；触发检查顺序固定 output_repeats → tool_calls → actions → rounds；作用域为主 agent + 检索子代理 ×2 三实例各自独立计数、触发同作用问询、**三实例同清零**。**注记**：清零语义下轮次判定为"最坏 7 轮一次"（其他判定点触发会清零推迟轮次触发）；原始 07-26 的"固定间隔定时器不受冷却限制"语义（每 7 轮必定触发）未采纳——若后续需要"必触发保证"可再以新 ADR 改定时器语义。
- **内容（原始文案原文，中性定位标记）**：

```text
[ORIENTATION_CHECKPOINT v0.1]
当前正在做什么？
当前任务定位是什么？
下一步输出应该服务哪个用户目标？
[/ORIENTATION_CHECKPOINT]
```

- **约束**：格式明显区别于用户/系统消息，使模型识别为"中途引导/定位标记"而非"新任务/质疑"；**不要求模型"停止当前推理并重新思考"**，而是"在当前位置做一个轻量定位标记"；允许问题还包括"当前可用来源、权限和输出边界是什么"；回答无控制流后果（证据记录，循环自然继续）；若实施结构解析，响应仅允许 `orientation_summary` / `current_task_position` / `next_output_target` 三字段，拒绝 counterexample_candidate / claim_disposition 等字段（原始 `verify_orientation_response` 语义）。
- **注记（token 门控）**：原始设计另有单轮公开输出 token 门控（`orientation_trigger_tokens: 7000`）——本次修订先不并入 5 判定点（需轮内输出 token 度量链路），语义保留于此，实施时以新 ADR 裁决是否加入。

### 4.6.3 信息充分性问询（原始设计严格恢复——与 5 判定点解绑，无简化）

- **职责**：信息收集充分性确认——检索/信息获取后的轻量确认，防"信息不足即收尾"。
- **触发时机**：子代理检索或任何外部信息获取动作完成后、模型给出正式回复前（原始设计 `info_sufficiency_after_retrieval: true`）。**不绑定 5 判定点**。
- **触发条件（硬门控三条件）**：① 检索/信息获取动作已返回结果（tool_completed for retrieval/info-gathering tools）② 模型尚未发出面向用户的最终回复 ③ 动作间隙注入。
- **内容（原始文案原文，机械可验证事实——2026-08-09 用户指示严格恢复，无任何简化）**：

```text
[INFO_SUFFICIENCY_CHECK v0.1]
已获取来源: {source_count} 项
覆盖范围: {categories}
全文可见: {fulltext_count}/{source_count}
缺失: {missing_categories}

当前信息是否足够回答用户问题？
如不足，还需哪些信息？
[/INFO_SUFFICIENCY_CHECK]
```

- **约束**：绝不暗示"你应该质疑当前结论"；只陈述**机械可验证**的事实（来源数量、类别、全文可见性）；不可作为"模型已经查过了"的证明（证据在 gate receipt）；不允许模型用此机制绕过 source visibility gate 的 block/defer 决策。定稿简化版（"本轮检索已完成"）**彻底废弃**——与机械门禁"只陈述机械可验证事实"约束冲突。
- **数据出处**：机械事实字段（source_count / categories / fulltext_count / missing_categories）来自子代理结构化结果——`source_ledger`（每项来源的全文可见性状态）+ `query_summary`（实际执行的每次检索动作）+ `filtering_log`（被排除内容及原因）（`RETRIEVAL_SUBAGENT_AUDIT` "结果透明"原文）。
- **注记（2026-08-09 治本方向 + 子代理生命周期实现偏差，讨论中未定稿）**：
  - **子代理生命周期设计原文**（`docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`）：子代理对话由**主代理打开（派遣合同）和关闭（确认本轮完毕）**——显式开关；**关闭不清零**：journal 完整保留，子代理可被同一 parent session **重新唤醒**；关闭回执（Session Close Receipt）：`conversation_zeroed: false` / `journal_preserved: true` / `can_resume: true` / `subagent_resumable: true`（需 parent_session_id 匹配 + 新合同）；子代理动作面 = **调用本地浏览器**（外部检索，LBR-001 local_browser 默认主路径）+ 项目文档（内部检索）。
  - **实现偏差（实锤）**：`agents/retrieval.rs` 实现为**瞬态单轮调用**（每次 `run_retrieval` 新会话、budget_turns=1 硬编码、无打开/关闭命令、无持久无唤醒无关闭回执）+ **零工具子代理**（`tools: Vec::new()`——无法调用浏览器/任何工具，检索内容纯模型生成）+ **自由文本结果**（[DOC]/[来源] 行解析替代结构化 source_ledger/query_summary）。与设计原文全面不符。
  - **治本方向（用户 2026-08-09 提出）**：主 agent 发出**关闭子代理命令**后触发机械门禁——阶段 1 先注入信息充分性问询（问主 agent），等待主 agent 回复后阶段 2 触发**关闭选项**（yes → 放行关闭；no/uncertain → 不关闭可继续）。子代理须为**单轮检索任务内持久化 + 显式开关**（按生命周期设计原文），非瞬态。定稿前开放问题：持久会话形态、显式开关工具面、结构化结果 schema、浏览器动作接线。

### 4.6.4 子代理 completion check（保留，中性确认）

关闭检索工具/检索子代理前的中立完成确认——一次性（Python 先例 `RETRIEVAL_COMPLETION_CHECK v0.1` 逐字移植）；block 注入子代理请求，响应逐行解析 yes/no/uncertain（`parse_completion_decision`），事件带完整响应文本作证据。**中性策略**：claim_policy 全部 False（may_generate_counterexample_candidate / may_set_claim_disposition / may_enter_global_review / may_request_new_subagent 全 False，claim_strength_effect "none"）——不评估正确性、不进入全局审查、不请求新子代理；no/uncertain 只列出还需要的内容类型。

### 4.6.5 反例询问（保留定稿语义 + 职责边界明示）

1. **正式答案输出前不触发方向/信息问询**（过犹不及）：模型能力足够，多加只是负担；且输出前补救窗口（"还能补收集"）已关闭，价值近零。
2. **正式答案输出前的反例询问仅触发一次**，message_block 显式告知"本询问仅出现一次"——与轮内问询区分收尾 gate 语义。
3. **plan 写入前的反例询问保留**——plan approval gate 链组成部分，不在"仅一次"范围内。
4. **中途不触发是设计本意**：自我质询没必要多（单一主 agent）；**中途方向维护由方向问询（§4.6.2）承担，收尾自查由反例询问承担**——职责分工不混淆；机械化门禁不随便扩大范围。

### 4.6.6 停滞守卫（独立，机械化门禁不扩大范围）

公开输出连续重复 / n-gram 重复 > 10 触发 `restart_requested`（全局重启语义，重置一切，与清零不冲突）。独立机械门禁，不参与任何问询判定；"输出重复/循环"类失效归它，不归方向/信息问询。

### 4.6.7 子代理动作语义分层（保留 2026-08-04 语义）

- 主 agent 与检索子代理除职责、prompt、权限外配置一致且各自独立（Two-Subagent Default Constraint 精神）。
- 主 agent：底层工具级事件（ToolDispatcher 包装）。
- 子代理：上层语义动作——一次 run_retrieval（一个 goal 执行）= 1 个动作；budget_turns > 1 时每轮 = 1 个动作。子代理内部**关闭工具级计数**（检索本身动作多——search/fetch/read 多工具调用，工具级计数必然误触发）。
- 其余判定点（输出阈值/工具调用/轮次/工具种类）主 agent 与子代理照常（子代理工具调用恒 0——内部关闭；输出度量用其返回文本；工具种类在子代理内部不适用——其工具面固定）。

### 4.6.8 Block 文案模板（三套分开，2026-08-09 修订）

- 方向问询：`[ORIENTATION_CHECKPOINT v0.1]`（§4.6.2 原文）——"当前正在做什么 / 当前任务定位是什么 / 下一步输出应该服务哪个用户目标"。
- 信息充分性：`[INFO_SUFFICIENCY_CHECK v0.1]`——**原始文案全文严格恢复**（来源数/覆盖/全文可见/缺失类别；机械事实数据出处 = 子代理结构化结果 source_ledger/query_summary，§4.6.3）。定稿简化版（"本轮检索已完成"）废弃。
- 反例询问：`[COUNTEREXAMPLE_GATE v0.1]`（正式答案变体含"仅一次"行；plan 变体不含）。
- 子代理 completion check：`[RETRIEVAL_COMPLETION_CHECK v0.1]`（Python 逐字移植）。

### 4.6.9 实现位置（2026-08-09 修订版）

- 方向问询：判定点计数器 + 清零状态机 —— `orz-loop/src/inquiry.rs`（`InquiryCounters`/`InquiryThresholds`/`DEFAULT_THRESHOLDS`，**2026-08-09 起 4 判定点**：output_repeats >10 / tool_calls >10 / actions >10 / rounds >7——tool_variety 不实现）；检查在 `controller.rs` 工具循环后动作间隙（每工具轮后）；block（`ORIENTATION_CHECKPOINT_BLOCK`）以 Role::User 注入主 agent 下一轮；事件沿用 `neutral_inquiry`（payload `message_block` 区分机制），每轮机械 `orientation_checkpoint` 记录保留（Python parity）。
- 信息充分性：检索动作完成后触发 —— `controller.rs` run_retrieval_subagent 返回后；block 文案恢复原始机械事实模板（`INFO_SUFFICIENCY_BLOCK` 常量文案改回 `INFO_SUFFICIENCY_CHECK v0.1` 全文；机械事实数据出处 = 子代理结构化结果 source_ledger/query_summary，随子代理持久化重构落地）。
- 子代理 completion check：`agents/retrieval.rs` `completion_check_block` 参数 + `parse_completion_decision`。
- 反例询问：plan 写入前 + 正式答案输出前（无 tool_calls 分支拦截一次）——`run_plan` / 工具循环无 tool_calls 分支（`COUNTEREXAMPLE_GATE_BLOCK`，被拦截草稿 journal 为 model_output 但不进入会话）。
- 各 gate 记录 run-event：`neutral_inquiry`（方向问询 / 信息充分性按 message_block 区分）/ `counterexample_gate` / `retrieval_completion_check`（run-event schema 变体，payload schema 见 `runtime/`）；模型回答无控制流后果（证据记录，循环自然继续）。

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
**目标**：功能闭环达到 Windows pre-beta；不等于完整 Windows 兼容性或正式产品完成

1. orz-tui → assurance workbench
2. orz-assurance::session::snapshot
3. orz-assurance::sandbox::job_object, credential, permit
4. Codex 纪律：严格 Clippy，零死代码 crate
5. Python 项目 → reference-spec
6. **§4.6 接线**（2026-08-04 审查补列；2026-08-04 已接线，部分闭合；**2026-08-09 修订语义——两机制分离恢复**）：反例询问（plan 写入前 + 正式答案输出前**仅一次 + message_block 显式告知**）——`run_plan` submit_plan 前模型轮（plan 变体无"仅一次"行）+ 工具循环无 tool_calls 分支拦截一次；**方向问询**（ORIENTATION_CHECKPOINT，绑定 4 判定点计数/清零状态机——2026-08-09 起 output>10/tool_calls>10/actions>10/rounds>7，tool_variety 已删，阈值定稿见 §4.6.2/ADR-0005）；**信息充分性**（INFO_SUFFICIENCY_CHECK，独立开关——检索/信息获取动作后触发，不绑判定点，§4.6.3）；子代理关闭前 completion check（Python `RETRIEVAL_COMPLETION_CHECK` 移植，逐字）；各机制均记 run-event（neutral_inquiry 按 message_block 区分机制 + counterexample_gate + retrieval_completion_check，payload schema 见 runtime/，Python authority 先行）。OrzHost + PermissionBridge 接入三入口已于 Slice #1 完成。已知边界：模型回答暂不驱动控制流（evidence-only）；plan 变体 gate 在机械 plan 下为证据性触发；**2026-08-09 起实现与设计偏差待修**（现行实现为畸形拼合：判定点+每工具轮检查绑 INFO_SUFFICIENCY 文案；方向问询仅事件记录无模型交互；子代理瞬态零工具——实施排期见 memory/索引）
7. **验证**: 全功能 conformance suite

### Phase 4: Windows beta 加固 + 案例库
**目标**：用真实 Windows 任务和 beta 环境把“Windows-first 定位”逐步转化为可复现、可回归、可披露边界的工程证据

1. 建立追加式事故台账，保留未定位、不可复现和非 Windows 特有的反例；
2. 按 `windows_native` / `cross_platform_agent` / `harness_environment` 分类并维护脱敏精选案例库；
3. 将稳定案例晋级为自动回归测试或有明确步骤的人工复核；
4. 记录 Windows/Shell/项目工具链环境矩阵，不以单机通过推广兼容声明；
5. SWE-bench-Live/Windows 用于解题能力和 debug，Windows 案例闭环指标单独统计；
6. 正式发布前冻结已验证环境、已知限制、案例入口与测试证据。

详细契约：`WINDOWS_RUNTIME_CONTRACT_v0.1.md` §6。

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
