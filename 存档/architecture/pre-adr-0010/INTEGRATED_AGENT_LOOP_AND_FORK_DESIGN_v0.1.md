# 整合 Agent Loop 与 Fork 实现设计 v0.1

> Archive metadata: original_path=`architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.1.md`; archived_at=`2026-08-09`; final_status=`transferred/superseded`; superseded_by=`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`; authority=`historical design input only`.


> **已被 v0.2 取代 (2026-08-03)**：[`INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md`](INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md) —
> 融合架构（Codex 纪律 + Grok 能力）。v0.1 的 LoopHost trait 和注入点分类被保留，
> 但 orz-shell + Grok TUI 被删除，orz-host 替代 shell 成为新的薄核心。

状态：2026-08-03。本文整合三份核心设计文档 + 对 `B:\orz` fork 代码库的直接审查，
消除矛盾，成为后续实现的唯一真相源：

- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md) — 五源融合设计语言
- [`FORK_IMPLEMENTATION_DESIGN_v0.1.md`](FORK_IMPLEMENTATION_DESIGN_v0.1.md) — fork 实现设计（内容标题 v0.2）
- [`AGENT_LOOP_REDESIGN_v0.1.md`](AGENT_LOOP_REDESIGN_v0.1.md) — Agent Loop 重设计（Pro/Flash + 黑板 + MechanicalRelay）
- `B:\orz` 仓库 `feat/phase1-scaffold` @ `a2a7868` — 实际 fork 代码，74 crate workspace

## 核心定位

**orz 从 Grok Build 取用成熟底座，自研 Agent Loop 引擎。不重复实现可用的成熟组件。**

```
Grok 提供（成熟底座 — 保留）：       orz 自研（编译进二进制）：
  orz-tools       (工具执行)           Agent Loop Controller (~1,050 行)
  orz-workspace   (file/VCS)           ├─ PromptBuilder       (TOOL_AVAILABILITY, CHECKPOINT, SUFFICIENCY)
  orz-sandbox     (Job Object)         ├─ ModelGateway        (thinking:disabled 内置)
  orz-mcp         (MCP 协议)           │   └─ transport/      (← 保留 Grok SSE streaming + retry ~3k 行)
  orz-http        (HTTP 客户端)        ├─ ToolDispatcher      (IPG + Snapshot + counters — 原生)
  orz-markdown    (终端渲染)           ├─ OrientationMonitor  (事件驱动触发 + cooldown)
  orz-config      (配置)               ├─ JournalWriter       (hash-chained JSONL)
  orz-hooks       (hook 系统)          │
  orz-secrets     (凭据脱敏 — 保留)    ├─ MechanicalRelay     (纯 function.name 路由)
  orz-auth        (凭据管理 — 保留)    ├─ Blackboard          (Plan/Exec/Ret×2/GateLog 分区)
  orz-chat-state  (持久化 — 保留)      ├─ Pro Agent           (规划+审查)
  orz-subagent-   (子代理传输 — 保留)  ├─ Flash Agent         (执行+决策)
    resolution                         └─ Retrieval Subagents (内部+外部)
  ACP 协议        (JSON-RPC stdio)
  Grok TUI        (兜底 UI)             LoopHost trait ← orz-shell 实现
                                        (工具注册/权限/持久化/MCP/compaction — 单向依赖)

Grok Sampler actor (~8k 行) → 删除，被 Agent Loop Controller 替代
Grok Sampler transport (~3k 行) → 保留，移入 orz-loop::gateway::transport
Grok Session Actor turn machinery (~34k 行) → feature-gate 后逐步删除
Grok telemetry → hard no-op（编译时 feature gate，不靠配置开关）
```

Agent Loop Redesign **不是往 Grok 循环里"注入"**——它是替代 Grok Sampler + Session Actor turn machinery
的独立引擎。从 Grok 保留的包括：工具执行、workspace、沙箱、MCP、HTTP 基础设施（含 SSE streaming/retry）、
markdown、config、凭据管理、持久化、子代理传输、ACP 协议、Grok TUI 兜底。

---

## 1. 架构全景

```
┌──────────────────────────────────────────────────────────────────────┐
│                          orz 二进制                                    │
│                                                                       │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │              自研核心：orz-loop crate                              │ │
│  │                                                                   │ │
│  │  while True:                                                      │ │
│  │      context = PromptBuilder.build(system_prompt, tool_defs,      │ │
│  │                                    blackboard, pending_prefix)    │ │
│  │      response = ModelGateway.stream(context)                      │ │
│  │      │                                                            │ │
│  │      │  ModelGateway 内置:                                        │ │
│  │      │  · thinking: disabled (IP1 原生)                           │ │
│  │      │  · transport/ ← 保留 Grok SSE streaming + retry (~3k 行)  │ │
│  │      │                                                            │ │
│  │      for tool_call in response.tool_calls:                        │ │
│  │          route = MechanicalRelay.route(tool_call.function.name)   │ │
│  │                                                                   │ │
│  │          match route:                                             │ │
│  │              DispatchToFlash      → Flash.execute()               │ │
│  │              DispatchToSubagent   → subagent.dispatch()           │ │
│  │              TriggerGate          → assurance.evaluate()          │ │
│  │              WritePlan            → blackboard.plan.update()      │ │
│  │              WriteExecSection     → blackboard.exec.append()      │ │
│  │              Passthrough          → ToolDispatcher.execute()      │ │
│  │                  │                                                │ │
│  │                  │  ToolDispatcher 原生 (不注入 orz-tools):        │ │
│  │                  │  · pre:  IPG evaluate (IP3a)                   │ │
│  │                  │  · pre:  SnapshotStore::track() (IP5)          │ │
│  │                  │  · exec: LoopHost.call_tool() → orz-tools      │ │
│  │                  │  · post: trigger.record_* (IP3b)               │ │
│  │                  │  · post: sufficiency trigger (IP3c)            │ │
│  │                                                                   │ │
│  │      if OrientationMonitor.should_fire():                         │ │
│  │          pending_prefix = checkpoint.message_block (IP2b 原生)    │ │
│  │                                                                   │ │
│  │  ┌────────────────────────────────────────────┐                  │ │
│  │  │              Blackboard                      │                  │ │
│  │  │  PlanSection(Pro写)  ExecSection(Flash写)   │                  │ │
│  │  │  InternalRet(内部子代理)  ExternalRet(外部) │                  │ │
│  │  │  GateLog(Assurance写)                       │                  │ │
│  │  │  读取：所有 Agent 可读  写入：每区唯一写入者 │                  │ │
│  │  └────────────────────────────────────────────┘                  │ │
│  │                                                                   │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐           │ │
│  │  │ Pro      │  │ Flash    │  │ Retrieval Subagents  │           │ │
│  │  │ 规划+审查 │  │ 执行+决策│  │ 内部: ProjectDoc    │           │ │
│  │  │ DeepSeek  │  │ DeepSeek │  │ 外部: WebSearch处理  │           │ │
│  │  └──────────┘  └──────────┘  └──────────────────────┘           │ │
│  │                                                                   │ │
│  │  ┌────────────────────────────────────────────┐                  │ │
│  │  │  LoopHost trait (orz-loop 定义, orz-shell 实现)               │ │
│  │  │  tools_registry()  call_tool()  request_permission()          │ │
│  │  │  persist_turn()    trigger_compaction()  run_hooks()          │ │
│  │  │  auth_credentials()  mcp_tools()                              │ │
│  │  │  → orz-loop 永远不 import orz-shell                           │ │
│  │  └────────────────────────────────────────────┘                  │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                   │                                    │
│       ┌───────────────────────────┼───────────────────────┐           │
│       ▼                           ▼                       ▼           │
│  ┌──────────┐  ┌─────────────┐  ┌─────────────────┐                 │
│  │orz-shell │  │ orz-tools   │  │ orz-workspace   │                 │
│  │(ACP 入口)│  │ (不改!)     │  │ (不改!)         │                 │
│  │IP4a-c    │  │ kept Grok   │  │ kept Grok       │                 │
│  │IP6 注入  │  │             │  │ IP6 注入        │                 │
│  │kept Grok │  │             │  │                 │                 │
│  └──────────┘  └─────────────┘  └─────────────────┘                 │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │         orz-assurance (编译进二进制，不是外部 sidecar)            │ │
│  │  gates/  orientation/  journal/  session/  sandbox/              │ │
│  │  credential/  permit/                                            │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │  保留的 Grok 成熟底座 (不改): orz-tools, orz-workspace,          │ │
│  │  orz-sandbox, orz-mcp, orz-http, orz-markdown, orz-secrets,     │ │
│  │  orz-auth, orz-chat-state (持久化), orz-subagent-resolution     │ │
│  │  (传输层), Grok TUI pager* (兜底 UI)                            │ │
│  └─────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘

                        双二进制架构

         orz-bin                          xai-grok-pager-bin
         (orz TUI 入口)                   (Grok 原生 TUI 入口)
              │                                    │
         orz-tui                           xai-grok-pager
         (assurance workbench)             (Grok 原生 TUI)
              │                                    │
              ▼                                    ▼
            orz                                  grok
     "主 UI，日常使用"                    "兜底 UI，紧急备用"

共享完全相同的 MvpAgent → orz-loop → orz-assurance。
区别仅在 UI 层。grok TUI 不展示 assurance 面板，但 gate/journal 静默运行。
grok TUI 在进程内运行 MvpAgent（`spawn.rs:200-245`），不需要额外 wiring。
```

---

## 2. 注入点 / Assurance 功能重新分类

13 个 assurance 功能按照"属于循环策略还是约束 kept 组件"分为三类。

### 2.1 原生循环代码（orz-loop 内部，不称"注入"，不修改 kept Grok 组件）

| ID | 功能 | 归属组件 | 说明 |
|----|------|---------|------|
| IP1 | `thinking: disabled` | **ModelGateway** | DeepSeek body 固定 `{"thinking": {"type": "disabled"}}`。删除 `scripts/deepseek_thinking_proxy.py`。保留 Grok SSE streaming + retry 传输层（`orz-sampler` 的 `client/stream/retry/events/types` → `orz-loop::gateway::transport/`）。 |
| IP2a | `[TOOL_AVAILABILITY]` | **PromptBuilder** | `build()` 末尾追加 context block，会话开始时注入一次。 |
| IP2b | `[ORIENTATION_CHECKPOINT]` | **Agent Loop 主循环** | `while` 中 `OrientationMonitor.should_fire()` → 注入下一轮 system prompt prefix。 |
| IP2c | `[INFO_SUFFICIENCY_CHECK]` | **PromptBuilder** | 检索后 `pending_sufficiency_check == true` → 下一轮 system prompt 前缀追加。 |
| IP3a | IPG gate（指令来源检查） | **ToolDispatcher** | `execute()` 内部，调用 `orz-tools` **之前**做 IPG evaluate。orz-tools 本身不修改。 |
| IP3b | orientation counters | **ToolDispatcher** | `execute()` 内部，调用 `orz-tools` **之后**更新 trigger 计数器。 |
| IP3c | sufficiency trigger | **ToolDispatcher** | 检索工具执行后标记 `pending_sufficiency_check`。 |
| IP5 | pre-mutation snapshot | **ToolDispatcher** | `execute()` 内部，文件修改工具前调用 `SnapshotStore::track()`。orz-workspace 本身不修改。 |

以上在 `orz-loop` 中实现，不标记 `ORZ-INJECTION`，不修改 kept 组件。

**IP3/IP5 的关键决策**：不在 `orz-tools/src/bridge.rs` 或 `orz-workspace` 中注入——
gate 排序、snapshot 时机是循环策略，由 `orz-loop::ToolDispatcher` 在调用 kept 组件前后包装执行。
这比修改 kept 组件更干净，且两个二进制自动获得相同行为。

### 2.2 注入点（修改 kept Grok 组件，标记 `ORZ-INJECTION`）

仅剩 3 个注入点，都在 orz-shell 的 ACP 入口：

| ID | 注入目标 | 精确位置 | 注入内容 |
|----|---------|---------|---------|
| IP4a | `orz-shell` | `acp_agent.rs` `new_session()` | trust verify → journal create → `run_preflight` event |
| IP4b | `orz-shell` | `acp_agent.rs` `prompt()` 入口 | `trigger.next_turn()` |
| IP4c | `orz-shell` | session 关闭路径 | `run_finished` event → journal seal → receipt |

注入代码统一使用标记：
```rust
// ── ORZ-INJECTION: <name> (IP<N>) ──
// Replaces: <旧 workaround (如有)>
// <注入代码>
// ── END ORZ-INJECTION ──
```

### 2.3 架构不变量（对 kept 组件的约束，由编译时类型系统 + 启动时 assert 保证）

**IP6 hard-gate enforcement** — 注入到 `orz-workspace/src/permission/` 的最终决策点 +
`orz-hooks` observe-only 标记：

1. hook 返回值只能从 allow 降级到 deny/ask，不能从 deny 升级到 allow
2. `GateDecision::Block` 不可被 hook 或用户偏好覆盖
3. 所有 permission decision 写入 journal event（通过 LoopHost 注册的 observer callback）
4. `SecurityFinding.confidence == heuristic` 时不能自动产生 Block（Goose 原则）
5. `SecurityFinding` 不自动产生 `PermissionDecision`（Goose 原则）

IP6 是唯一需要修改 kept Grok permission 代码的 assurance 功能——
因为 permission/approval pipeline（manager, prompter, interactive approval ledger）
是成熟底座的一部分，拥有用户交互通道。不变量在其最终决策点强制执行，
确保任何调用方（loop, MCP, hook, plugin）都无法绕过。

---

## 3. Crate 矩阵

### 3.1 KEEP — 直接采用，不修改

成熟且与 assurance/loop 策略无关的 Grok 组件：

| Crate | 功能 | 为什么保留 |
|-------|------|-----------|
| `orz-tools` | 工具执行 | 成熟实现，不改——loop 通过 ToolDispatcher 包装调用 |
| `orz-tools-api` | 工具接口定义 | 类型定义 |
| `orz-workspace` | 文件系统/VCS | 成熟实现，不改——snapshot 由 loop ToolDispatcher 管理 |
| `orz-workspace-client` | workspace 客户端 | — |
| `orz-workspace-types` | workspace 类型 | — |
| `orz-sandbox` | OS sandbox 隔离 | Job Object 成熟实现 |
| `orz-mcp` | Model Context Protocol | MCP 协议支持 |
| `orz-http` | HTTP 客户端基础设施 | 成熟 HTTP client |
| `orz-markdown` / `orz-markdown-core` | 终端渲染 | 纯 UI |
| `orz-paths` / `orz-env` / `orz-shared` | 工具库 | 通用 |
| `orz-version` | 版本信息 | — |
| `orz-agent` | agent 基础类型 | — |
| `orz-secrets` | **凭据脱敏** | 成熟实现，保留。ModelGateway 需要 BYOK/session-token 管理 |
| `orz-auth` | **凭据管理** | 成熟实现，保留。DeepSeek API key 管理需要 |
| `orz-chat-state` | **持久化/compaction** | 成熟实现，保留。Blackboard 替代其 in-turn 状态角色，但持久化/compaction 仍是 loop 需要的 session service |
| `orz-subagent-resolution` | **子代理传输** | 保留传输层（子代理 spawn）。resolution 逻辑被 MechanicalRelay 替代，但传输通道保留 |
| `orz-shell-session-support` | managed-MCP helpers | Plan A 中恢复的编译依赖，保留 |
| `xai-agent-lifecycle` / `xai-file-utils` / `xai-fast-worktree` | 基础库 | — |
| `xai-fsnotify` / `xai-gix-status` / `xai-hooks-plugins-types` | 基础库 | — |
| `xai-token-estimation` / `xai-tty-utils` / `xai-system-power` | 基础库 | — |
| `xai-prompt-queue` / `ptyctl` / `ptyctl-cli` | 基础库 | — |
| `xai-ratatui-inline` / `xai-ratatui-textarea` | TUI 组件 | — |
| `xai-tracing-macros` / `xai-crash-handler` | 诊断 | — |
| `xai-workflow` / `xai-hunk-tracker` | 工作流 | — |
| `xai-acp-lib` | ACP gateway/channels | 协议层，不改 |
| `xai-grok-pager` / `pager-render` / `pager-minimal` / `pager-bin` | **Grok TUI 兜底** | 保留裁剪版（去 voice/update/mermaid/announcements 引用） |

### 3.2 MODIFY — 保留骨架，最小修改

| Crate | 修改 | 说明 |
|-------|------|------|
| `orz-shell` | `acp_agent.rs`: IP4a-c 注入；`init.rs`: telemetry 调用替换为注释；dispatch seam (line 2507): `SessionCommand::Prompt` → `LoopController::run_turn` | 仅改 ACP 入口 + telemetry 禁用。Turn machinery 保留在 `feature = "legacy-turn"` 后编译 |
| `orz-workspace` | `permission/manager.rs`: IP6 不变量执行 + journal observer callback 注册点 | 仅改 permission 最终决策点 |
| `orz-hooks` | observe-only 标记；IP6 降级不变量 | 小修改 |
| `orz-config` | 移除 remote-settings 引用 | — |
| `orz-models` | 默认模型列表 → orz profiles (DeepSeek V4 Pro/Flash) | — |
| `orz-memory` | `memory.enabled = false` 默认 + 开关 | Phase 1 已完成 |
| `xai-grok-pager-bin` | 裁剪 update/announcements/voice 引用 | — |

### 3.3 SPLIT — 保留成熟部分，删除不需要的部分

| Crate | 保留（移入 orz-loop） | 删除 |
|-------|---------------------|------|
| `orz-sampler` (11,373 行) | **transport**: `client.rs`, `stream/`, `retry.rs`, `events.rs`, `types.rs` (~3k 行 SSE streaming + retry — 成熟的 HTTP 基础设施) → `orz-loop/src/gateway/transport/` | **actor machinery**: `actor/`, `handle.rs`, `doom_loop.rs`, `config.rs`, `metrics.rs`, `attribution.rs`, `sampling_log.rs` (~8k 行 — 被 AgentLoopController/ModelGateway 替代) |

SPLIT 而非 DELETE 的理由：Grok 的 SSE streaming + retry 是经过生产验证的成熟代码，
重新实现等于丢掉用户明确要保留的"成熟底座"。Agent Loop Redesign 替换的是采样策略和循环逻辑，
不是 HTTP 传输。

### 3.4 REPLACE — 完全替换为自研

| 原组件 | 替换为 | 说明 |
|--------|--------|------|
| `orz-sampler` actor machinery (~8k 行) | **AgentLoopController** (~1,050 行) | orz-loop core |
| `orz-telemetry` (~13,600 行) | **hard no-op** (保留编译但不初始化) | Phase 0 编译时 feature gate |
| `orz-plugin-marketplace` (~5,600 行) | **no-op stub** | 参考 xai-mixpanel stub 模式 |
| `orz-announcements` (~400 行) | **no-op stub** → Phase 2 删除 | 低耦合，4 文件 |

### 3.5 ADD — 全新

| Crate | 内容 |
|-------|------|
| `orz-bin` | composition root，产出 `orz` 二进制 |
| `orz-assurance` | gates + orientation + journal + session + sandbox + credential + permit (Python spec → Rust) |
| `orz-tui` | assurance workbench TUI (Python `assurance/tui/` → Rust) |
| `orz-loop` | AgentLoopController + MechanicalRelay + Blackboard + PromptBuilder + ModelGateway(含 transport/) + ToolDispatcher + OrientationMonitor + JournalWriter + Pro/Flash + RetrievalSubagents + **LoopHost trait** |

### 3.6 REMOVE — 完全删除（Phase 1 已完成）

`orz-update`, `orz-voice`, `orz-mermaid`, `orz-codebase-graph`, `orz-pager-pty-harness`,
`orz-test-support`, `orz-shell-base` (合并到 orz-shell)

### 3.7 后续删除目标（Phase 2-3，先 feature-gate 后删除）

**策略**：不再重蹈 "先删后修 → 981 错误" 的覆辙。所有删除对象先在 `feature = "legacy-turn"` 后面保留编译，
等新循环通过 conformance 后再物理删除。

| 目标 | 触发条件 |
|------|---------|
| Session Actor turn machinery (~34k LOC: `run_loop.rs` Prompt arm, `turn.rs`, `sampler_turn.rs`, `goal_*.rs`, `prompt_build.rs`, `tool_dispatch.rs`, `recap.rs`, `memory_dream.rs`, `laziness*.rs`, `interjection.rs`, `rewind.rs`, `two_pass.rs`, `stop_gate.rs`) | Phase 2 单 Agent loop 通过 conformance |
| `orz-announcements` | Phase 2（低耦合，4 文件） |
| `orz-plugin-marketplace` stub → 最终删除 | Phase 3 |
| `orz-telemetry` (82 个引用点消除后) | Phase 3 |

---

## 4. orz-loop 设计

### 4.1 Crate 边界：LoopHost trait

`orz-loop` 永远不 `import orz-shell`。循环需要的 session services 通过 trait 抽象：

```rust
// orz-loop/src/host.rs

/// 由 orz-shell 的 Session Services 实现。
/// orz-loop 只依赖此 trait，不依赖 orz-shell。
#[async_trait]
pub trait LoopHost: Send + Sync {
    /// 返回当前可用的工具注册表（含 MCP 工具）。
    fn tools_registry(&self) -> &ToolRegistry;

    /// 执行工具调用。ToolDispatcher 通过此方法调用 kept orz-tools。
    async fn call_tool(&self, name: &str, args: Value, call_id: &str)
        -> Result<ToolResult, ToolError>;

    /// 请求用户授权高风险动作。通过 ACP ApprovalRequest 通道。
    async fn request_permission(&self, risk: RiskClass, tool: &str, args: &Value)
        -> Result<PermitDecision, PermitError>;

    /// 持久化当前 turn 到 orz-chat-state。
    async fn persist_turn(&self, turn: &TurnRecord) -> Result<(), PersistError>;

    /// 触发上下文压缩（compaction）。
    async fn trigger_compaction(&self) -> Result<(), CompactionError>;

    /// 执行 hooks（标记了 observe-only）。
    async fn run_hooks(&self, event: &HookEvent) -> Vec<HookResult>;

    /// 获取认证凭据（DeepSeek API key 等）。
    fn auth_credentials(&self) -> Result<Credentials, AuthError>;

    /// 获取 MCP 工具列表。
    fn mcp_tools(&self) -> Vec<ToolDef>;
}
```

orz-shell 的 `MvpAgent` 持有 `Arc<dyn LoopHost>` 实现，在构造 `AgentLoopController` 时注入。

### 4.2 模块结构

```
crates/orz-loop/
├── Cargo.toml
└── src/
    ├── lib.rs                    # 模块注册 + 公共导出
    ├── host.rs                   # LoopHost trait 定义
    ├── controller.rs             # Agent Loop Controller 主 while 循环
    ├── prompt_builder.rs         # system prompt + IP2a/IP2c assurance blocks
    ├── model_gateway.rs          # DeepSeek API + thinking:disabled (IP1)
    │   └── transport/            # ═══ 保留 Grok SSE streaming + retry (~3k 行) ═══
    │       ├── mod.rs
    │       ├── client.rs         # ← orz-sampler client.rs
    │       ├── stream.rs         # ← orz-sampler stream/
    │       ├── retry.rs          # ← orz-sampler retry.rs
    │       ├── events.rs         # ← orz-sampler events.rs
    │       └── types.rs          # ← orz-sampler types.rs
    ├── tool_dispatcher.rs        # ToolDispatcher: IP3a-c + IP5 原生
    ├── orientation_monitor.rs    # OrientationTrigger + cooldown 管理 (IP2b)
    ├── journal_writer.rs         # journal 事件记录 (delegate to orz-assurance)
    ├── mechanical_relay.rs       # 纯 function.name 路由表
    ├── blackboard.rs             # PlanSection / ExecSection / InternalRet / ExternalRet / GateLog
    ├── pro_agent.rs              # Pro agent 配置 + 角色 prompt
    ├── flash_agent.rs            # Flash agent 配置 + 角色 prompt + 授权流程
    └── retrieval/                # 检索子代理
        ├── mod.rs
        ├── internal.rs           # 内部文档检索子代理
        └── external.rs           # 外部搜索处理子代理
```

### 4.3 Cargo.toml

```toml
[package]
name = "orz-loop"
version = "0.1.0"
edition = "2024"
license = "Apache-2.0"

[dependencies]
orz-assurance = { path = "../orz-assurance" }
# 注意: orz-loop 不依赖 orz-shell —— 通过 LoopHost trait 解耦
orz-tools-api = { path = "../orz-tools-api" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
async-trait = "0.1"
uuid = { version = "1", features = ["v4"] }
thiserror = "2"
tracing = "0.1"
```

### 4.4 orz-shell 的修改

orz-shell 不再包含 agent loop。它负责 ACP 协议 + 实现 LoopHost + IP4 注入 + IP6 permission 注入：

```rust
// orz-shell/src/agent/mvp_agent/acp_agent.rs
use orz_loop::{AgentLoopController, LoopHost};

pub struct MvpAgent {
    loop_controller: AgentLoopController,
    session_services: Arc<SessionServices>,  // implements LoopHost
}

impl acp::Agent for MvpAgent {
    async fn new_session(&self, arguments: acp::NewSessionRequest)
        -> Result<acp::NewSessionResponse, acp::Error>
    {
        // ── ORZ-INJECTION: session-lifecycle (IP4a) ──
        let trust_receipt = orz_assurance::session::trust::verify(
            &arguments.cwd, &self.config)?;
        let journal = orz_assurance::journal::recorder::JournalRecorder::new(
            &run_root.join("events.jsonl"))?;
        journal.record(RunEvent::run_preflight(&run_manifest, &trust_receipt)).await?;
        // ── END ORZ-INJECTION ──

        // 原有 new_session 逻辑 ... (保留 Grok session 初始化)
    }

    async fn prompt(&self, arguments: acp::PromptRequest)
        -> Result<acp::PromptResponse, acp::Error>
    {
        // ── ORZ-INJECTION: turn-lifecycle (IP4b) ──
        self.loop_controller.next_turn();
        // ── END ORZ-INJECTION ──

        // 委托给 orz-loop（替代原来的 SessionCommand::Prompt dispatch）
        let result = self.loop_controller.run_turn(
            arguments,
            Arc::clone(&self.session_services) as Arc<dyn LoopHost>,
        ).await?;

        // ── ORZ-INJECTION: session-close (IP4c) ──
        // (在 session 关闭路径中)
        self.journal.record(RunEvent::run_finished(...)).await?;
        self.journal.seal().await?;
        let receipt = self.journal.build_receipt()?;
        // ── END ORZ-INJECTION ──

        Ok(result)
    }
}
```

---

## 5. 解决已知矛盾

| 矛盾 | 裁决 | 依据 |
|------|------|------|
| Grok TUI 去留 | **保留为兜底**（双二进制架构） | 实现设计 v0.2 |
| Snapshot 归属 Phase | **Phase 4** | 实现设计 v0.2 / Agent Loop Redesign §9 |
| IP1 锚定矛盾 | **ModelGateway 内置**；sampler transport 保留移入 orz-loop | SPLIT 策略 |
| IP3/IP5 锚定矛盾 | **orz-loop ToolDispatcher 原生**，不注入 orz-tools/workspace | 循环策略不应修改 kept 组件 |
| Crate 命名 | 统一 **`orz-*`**，Grok TUI crates 保留 `xai-grok-*` | 与已执行代码一致 |
| GateDecision 命名 | 统一 **`Pass / Warn / Block / Defer / NotApplicable`** | 实现设计 v0.2+ |
| orz-secrets/orz-auth 去留 | **保留**（凭据管理底座） | ModelGateway 需要 BYOK/session-token |
| orz-chat-state 去留 | **保留**（持久化/compaction 底座） | Blackboard 替代 in-turn 角色，持久化仍需要 |
| orz-subagent-resolution 去留 | **保留传输层**（resolution 被 Relay 替代） | 子代理 spawn 通道保留 |
| orz-sampler 去留 | **SPLIT**：留 transport (~3k)，删 actor (~8k) | 不重复实现成熟的 SSE streaming |

---

## 6. 修正 4 个伪代码 Bug（来自 Post-Plan A Review §2.3）

| # | Bug | 修正 |
|---|-----|------|
| 1 | `SnapshotStore` sync 方法内用 `.await` — 无法编译 | 所有方法改为 `async fn` |
| 2 | `JournalRecorder::record()` 用 `try_send` — channel full 时静默丢事件，破坏 hash chain | 改为 `send().await` 阻塞发送；`record()` 改为 `async fn` |
| 3 | `orphan_events` 维度永无法填充（`RunEvent` 无 `turn_id`）— 死代码 | 从 `JournalInvariants` 中移除 `orphan_events`；三项不变量足够（dup/timestamp/hash） |
| 4 | verifier 不重算 `event_sha256` — payload 篡改无法检测 | 增加 `compute_event_sha256()` 步骤，逐 event 重算并与存储值对比 |

---

## 7. 实施阶段

### Phase 0: 稳定化（先于任何新代码）
**目标**：消除已知风险，为后续 Phase 提供安全基线

1. **Telemetry 硬禁用**：`orz-telemetry` 添加 `hard-disable` feature（默认 on）；`client::init` 改为编译时 no-op；保留 `unified_log` 本地文件写入；环境变量 `GROK_TELEMETRY_ENABLED` → `ORZ_TELEMETRY_ENABLED`
2. **4 个伪代码 bug 修复**（见 §6）
3. **IP 锚定清理**：更新 IP 表，标记 IP3/IP5 为原生（删除 sampler/IP 自相矛盾锚定）
4. **版本命名修正**：`FORK_IMPLEMENTATION_DESIGN_v0.1.md` 内容标题从 v0.2 → v0.1
5. **验证**：`cargo build` 绿色（74 crate workspace）；journal 单元测试通过；zero 遥测外发

### Phase 1: Journal + Transport + 单 Agent Loop
**目标**：`orz -p "hello"` → 有效 `events.jsonl`，hash chain 连续

1. 实现 `orz-assurance::journal`（event, chain, recorder **阻塞 send**, verifier 含 event_sha256 重算）
2. 创建 `orz-loop` skeleton：`LoopHost` trait, `AgentLoopController`（单 Agent 版本）, `ModelGateway`
3. **Split orz-sampler**：transport (client/stream/retry/events/types) → `orz-loop/src/gateway/transport/`；删除 actor/handle/doom_loop/metrics/attribution/sampling_log；删除 orz-sampler crate
4. Wire the seam：`MvpAgent::prompt()` dispatch → `LoopController::run_turn()`；`SessionCommand::Prompt` arm → `#[cfg(feature = "legacy-turn")]`
5. ModelGateway 内置 IP1：DeepSeek body `thinking: disabled`
6. `JournalWriter` (IP4a/c) + `SessionBootstrap` (workspace trust, envelope, journal 创建)
7. **验证**：`orz -p "hello"` → 有效 `events.jsonl`，hash chain 连续，Python verifier 可独立验证；两个二进制均运行新 loop

### Phase 2: Blackboard + Relay + Pro/Flash + Subagents + Gates
**目标**：Pro + Flash + 检索子代理完整协作

1. 实现 `Blackboard`（5 分区）、`MechanicalRelay`（纯 function.name 路由）
2. Pro/Flash 双 Agent（对等配置，角色 Prompt 不同）、Flash 授权流程
3. Retrieval Subagents（内部 ProjectDoc + 外部 WebSearch 处理）；复用 `orz-subagent-resolution` 传输
4. 实现 `orz-assurance::gates`（ipg, tool_availability, source_visibility, adapter）
5. 实现 `orz-assurance::orientation`（checkpoint, stagnation, sufficiency）
6. `ToolDispatcher`：IPG pre-exec (IP3a), orientation counters (IP3b), sufficiency trigger (IP3c), snapshot track (IP5)
7. `PromptBuilder`：TOOL_AVAILABILITY (IP2a), INFO_SUFFICIENCY_CHECK (IP2c)
8. IP6：hard gate enforcement @ orz-workspace permission 决策点 + orz-hooks
9. **Phase 2 exit**：删除 `#[cfg(feature = "legacy-turn")]` 后面的 Session Actor turn machinery（~34k LOC）和 `orz-announcements`
10. **验证**：完整 Pro+Flash+子代理+gate 链；Python conformance for gates 通过

### Phase 3: TUI + Snapshot + Polish
**目标**：完整 orz 产品

1. 移植 Python `assurance/tui/` → `orz-tui`（assurance workbench）
2. 实现 `orz-assurance::session::snapshot`（OpenCode 模式 shadow Git，**async fn**）
3. 实现 `orz-assurance::session`（trust, envelope, lifecycle）
4. 实现 `orz-assurance::sandbox::job_object`
5. 实现 `orz-assurance::credential` (Windows Credential Manager)
6. 实现 `orz-assurance::permit` (one-shot action permit)
7. Grok TUI 兜底裁剪（voice/update/mermaid/announcements 引用）
8. 最终清理：评估删除 `orz-telemetry`（82 个引用点消除后）、`orz-plugin-marketplace` stub、`orz-shell-session-support`（合并或保留）
9. Python 项目（`D:\CLI`）标记 `reference-spec`
10. **验证**：orz 全功能通过 Python conformance suite；`grok` 二进制兜底可用

---

## 8. Telemetry 硬禁用方案

Post-Plan A Review 确认 `orz-shell/src/agent/init.rs:215` 中 `orz_telemetry::client::init(...)` 在共享初始化路径中实时运行，
82 个 shell 文件引用 `orz_telemetry`，仍使用旧 `GROK_TELEMETRY_ENABLED` 前缀。

**修正**：

```toml
# orz-telemetry/Cargo.toml
[features]
default = ["hard-disable"]
hard-disable = []   # client::init 和所有 upload 路径编译为 no-op
telemetry = []      # 仅在显式 opt-in 时启用真实遥测（评测/调试用）
```

```rust
// orz-telemetry/src/client.rs
#[cfg(feature = "hard-disable")]
pub fn init(config: TelemetryConfig) -> TelemetryHandle {
    // no-op — telemetry is disabled at compile time
    TelemetryHandle::noop()
}

// orz-shell/src/agent/init.rs:215 — 替换为:
// Telemetry disabled at compile time — see orz-assurance design §8
// (original: orz_telemetry::client::init(...))
```

环境变量 `GROK_TELEMETRY_ENABLED` → `ORZ_TELEMETRY_ENABLED`（即使 `hard-disable` 下也无作用）。

---

## 9. 关键风险

| 风险 | 缓解 |
|------|------|
| Session Actor turn machinery 删除 (~34k LOC) 可能破坏未发现的依赖 | `feature = "legacy-turn"` 保留编译直到新循环通过 conformance |
| orz-sampler transport 移入 orz-loop 可能遗漏内部依赖 | Phase 1 第一步执行，编译验证。transport 模块相对独立（纯 HTTP） |
| Pro/Flash 双 Agent 是行为变更叠加在循环替换之上 | Phase 1 单 Agent 先验证，Phase 2 再加双 Agent + relay |
| Retrieval Subagents 复用 Grok spawn 时 resolution 层被 bypass | Phase 2 显式测试子代理启动路径 |
| IP6 journal-observer callback 未注册时 decisions 静默绕过 journal | `new_session()` 中 assert observer 已注册，否则 panic（fail-fast） |

---

## 10. 参考

### 被整合的源文档
- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.1.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.1.md)
- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.2.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.2.md)
- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md)
- [`FORK_IMPLEMENTATION_DESIGN_v0.1.md`](FORK_IMPLEMENTATION_DESIGN_v0.1.md)
- [`AGENT_LOOP_REDESIGN_v0.1.md`](AGENT_LOOP_REDESIGN_v0.1.md)

### 审查与分析
- [`PHASE2_POST_PLANA_REVIEW_v0.1.md`](../../../architecture/PHASE2_POST_PLANA_REVIEW_v0.1.md)
- [`PHASE2_COMPILATION_STATUS_v0.2.md`](../../../architecture/PHASE2_COMPILATION_STATUS_v0.2.md)
- [`IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md`](IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md)

### 代码锚点（`B:\orz` @ `a2a7868`）
- `orz-shell/src/agent/mvp_agent/acp_agent.rs` — ACP surface + dispatch seam (line 2507)
- `orz-shell/src/session/acp_session_impl/run_loop.rs` — Prompt command arm (line 597)
- `orz-shell/src/agent/init.rs` — telemetry init (line 215)
- `orz-sampler/src/` — 待 split：留 client/stream/retry/events/types，删 actor/handle/doom_loop
- `orz-workspace/src/permission/` — IP6 执行点（manager.rs, policy.rs）
- `xai-grok-pager/src/acp/spawn.rs` — grok TUI 进程内 MvpAgent 运行 (line 200-245)

### 外部参考
- Codex CLI: `codex-rs/rollout/src/recorder.rs` (Apache-2.0) — JournalRecorder pattern
- Gemini CLI: `invariantChecker.ts` (Apache-2.0) — invariants pattern
- OpenCode: `snapshot/index.ts` (MIT) — shadow Git pattern
- Goose: `security_inspector.rs` (Apache-2.0) — SecurityFinding/PermissionDecision 分离
- Grok Build: `500129c7` (Apache-2.0)

### ADR
- ADR-0001: evidence-constrained kernel
- ADR-0003: runtime-neutral assurance kernel
