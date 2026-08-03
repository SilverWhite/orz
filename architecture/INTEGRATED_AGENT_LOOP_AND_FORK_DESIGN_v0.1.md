# 整合 Agent Loop 与 Fork 实现设计 v0.1

状态：2026-08-03。本文整合三份核心设计文档，消除矛盾，成为后续实现的唯一真相源：

- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md) — 五源融合设计语言
- [`FORK_IMPLEMENTATION_DESIGN_v0.1.md`](FORK_IMPLEMENTATION_DESIGN_v0.1.md) — fork 实现设计（内容标题 v0.2）
- [`AGENT_LOOP_REDESIGN_v0.1.md`](AGENT_LOOP_REDESIGN_v0.1.md) — Agent Loop 重设计（Pro/Flash + 黑板 + MechanicalRelay）

## 核心定位

**orz 从 Grok Build 取用成熟底座，自研 Agent Loop 引擎。**

```
Grok 提供（成熟底座）：               orz 自研（编译进二进制）：
  orz-tools    (工具执行)              Agent Loop Controller
  orz-workspace(file/VCS)             ├─ PromptBuilder       (TOOL_AVAILABILITY, CHECKPOINT, SUFFICIENCY)
  orz-sandbox  (Job Object)           ├─ ModelGateway        (thinking:disabled 内置)
  orz-mcp      (MCP 协议)             ├─ ToolDispatcher      (透传 orz-tools + IPG 注入)
  orz-http     (HTTP 客户端)          ├─ OrientationMonitor  (事件驱动触发 + cooldown)
  orz-markdown (终端渲染)             ├─ JournalWriter       (hash-chained JSONL)
  orz-config   (配置)                 │
  orz-hooks    (hook 系统)            ├─ MechanicalRelay     (纯 function.name 路由)
  ACP 协议     (JSON-RPC stdio)       ├─ Blackboard          (Plan/Exec/Ret×2/GateLog 分区)
  Grok TUI     (兜底 UI)              ├─ Pro Agent           (规划+审查)
                                      ├─ Flash Agent         (执行+决策)
                                      └─ Retrieval Subagents (内部+外部)

Grok Sampler (~10,000行) → 完全删除，被 Agent Loop Controller (~1,550行) 替代
Grok chat-state          → 完全删除，被 Blackboard 替代
Grok telemetry           → hard no-op stub（编译时不初始化，不靠配置开关）
```

Agent Loop Redesign **不是往 Grok 循环里"注入"**——它是替代 Grok Sampler 的独立引擎。
从 Grok 保留的只是成熟的外围组件（tools/workspace/sandbox/MCP/ACP协议/HTTP/markdown/config）。

---

## 1. 架构全景

```
┌──────────────────────────────────────────────────────────────────────┐
│                          orz 二进制                                    │
│                                                                       │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │              自研核心：Agent Loop Controller                       │ │
│  │                                                                   │ │
│  │  while True:                                                      │ │
│  │      context = PromptBuilder.build(system_prompt, tool_defs,      │ │
│  │                                    blackboard, pending_prefix)    │ │
│  │      response = ModelGateway.stream(context)                      │ │
│  │                                                                   │ │
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
│  │                                                                   │ │
│  │      if OrientationMonitor.should_fire():                         │ │
│  │          pending_prefix = checkpoint.message_block                │ │
│  │                                                                   │ │
│  │  ┌────────────────────────────────────────────┐                  │ │
│  │  │              Blackboard                      │                  │ │
│  │  │                                             │                  │ │
│  │  │  PlanSection       ExecSection              │                  │ │
│  │  │  (Pro 写)          (Flash 写)               │                  │ │
│  │  │  · goal            · results[]              │                  │ │
│  │  │  · steps[]         · observations[]         │                  │ │
│  │  │  · decisions       · errors[]               │                  │ │
│  │  │  · auth_grants     · auth_requests[]        │                  │ │
│  │  │                                             │                  │ │
│  │  │  InternalRet       ExternalRet     GateLog  │                  │ │
│  │  │  (内部子代理写)    (外部子代理写)  (Assurance)│                  │ │
│  │  │                                             │                  │ │
│  │  │  读取：全部区域对所有 Agent 可读              │                  │ │
│  │  │  写入：每个区域仅一个写入者                   │                  │ │
│  │  └────────────────────────────────────────────┘                  │ │
│  │                                                                   │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐           │ │
│  │  │ Pro      │  │ Flash    │  │ Retrieval Subagents  │           │ │
│  │  │ 规划+审查 │  │ 执行+决策│  │ 内部: ProjectDoc    │           │ │
│  │  │ 工具:全量 │  │ 工具:全量│  │ 外部: WebSearch处理  │           │ │
│  │  │ 派遣子代理│  │ 派遣子代理│  │                      │           │ │
│  │  │ 授权 Flash│  │ 请求授权 │  │ 独立 API key        │           │ │
│  │  │ DeepSeek  │  │ DeepSeek │  │ DeepSeek Flash      │           │ │
│  │  └──────────┘  └──────────┘  └──────────────────────┘           │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                   │                                    │
│       ┌───────────────────────────┼───────────────────────┐           │
│       ▼                           ▼                       ▼           │
│  ┌──────────┐  ┌─────────────┐  ┌─────────────────┐                 │
│  │orz-shell │  │ orz-tools   │  │ orz-workspace   │                 │
│  │(ACP 入口)│  │ (工具调度)   │  │ (文件系统/VCS)  │                 │
│  │IP4a-c    │  │ IP3a-c 注入 │  │ IP5 注入        │                 │
│  │kept Grok │  │ kept Grok   │  │ kept Grok       │                 │
│  └──────────┘  └─────────────┘  └─────────────────┘                 │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │         orz-assurance (编译进二进制，不是外部 sidecar)            │ │
│  │                                                                   │ │
│  │  gates/    orientation/    journal/    session/                   │ │
│  │  (IPG,     (checkpoint,    (event,     (trust, envelope,          │ │
│  │   tool_avail, stagnation,   chain,      lifecycle, snapshot)      │ │
│  │   src_vis,   sufficiency,   recorder,                            │ │
│  │   adapter)   trigger)       verifier)   sandbox/  credential/     │ │
│  │                                        (job_obj)  permit/         │ │
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

共享完全相同的底层（orz-shell + orz-tools + orz-workspace + orz-assurance）。
区别仅在 UI 层：orz-tui 展示 gate/joural/orientation，grok 静默运行 assurance。
```

---

## 2. 注入点重新分类

Agent Loop Redesign 将 6 个原始注入点分为三类：

### 2.1 内置功能（在 Agent Loop Controller 中实现，不称为"注入"）

| ID | 功能 | 归属组件 | 实现位置 |
|----|------|---------|---------|
| IP1 | `thinking: disabled` | **ModelGateway** | 构造 DeepSeek Chat Completions body 时固定 `{"thinking": {"type": "disabled"}}`。删除 `scripts/deepseek_thinking_proxy.py`。 |
| IP2a | `[TOOL_AVAILABILITY]` context | **PromptBuilder** | `build()` 末尾追加 tool availability block。会话开始时注入一次。 |
| IP2b | `[ORIENTATION_CHECKPOINT]` | **Agent Loop 主循环** | `while` 循环中工具执行后 → `OrientationMonitor.should_fire()` → 注入到下一轮 system prompt prefix。 |
| IP2c | `[INFO_SUFFICIENCY_CHECK]` | **PromptBuilder** | 检索后 `pending_sufficiency_check == true` 时，下一轮 system prompt 前缀追加。 |

以上不再标记 `ORZ-INJECTION` 注释——它们是 orz-loop 的原生代码，不是对外部组件的修改。

### 2.2 注入点（修改 kept Grok 组件，标记 `ORZ-INJECTION`）

| ID | 注入目标 | 精确位置 | 注入内容 |
|----|---------|---------|---------|
| IP3a | `orz-tools` | `ToolBridge::call()` 执行前 | IPG evaluate → Block/Warn/Pass 决策 |
| IP3b | `orz-tools` | `ToolBridge::call()` 执行后 | `trigger.record_action()` + `record_tool_type()` |
| IP3c | `orz-tools` | 检索工具执行后 | `trigger.pending_sufficiency_check = true` |
| IP4a | `orz-shell` ACP | `new_session()` | trust verify → journal create → `run_preflight` event |
| IP4b | `orz-shell` ACP | `prompt()` 入口 | `trigger.next_turn()` |
| IP4c | `orz-shell` ACP | session 关闭路径 | `run_finished` event → journal seal → receipt |
| IP5 | `orz-workspace` | 文件修改工具 hook | `SnapshotStore::track()` → journal record |

注入代码统一使用标记：
```rust
// ── ORZ-INJECTION: <name> (IP<N>) ──
// Replaces: <旧 workaround (如有)>
// <注入代码>
// ── END ORZ-INJECTION ──
```

### 2.3 架构不变量（设计约束，不是代码注入）

**IP6 hard-gate enforcement** — 以下约束由编译时类型系统 + 启动时 assert 保证：

1. hook 返回值只能从 allow 降级到 deny/ask，不能从 deny 升级到 allow
2. `GateDecision::Block` 不可被 hook 或用户偏好覆盖
3. 所有 permission decision 写入 journal event
4. `SecurityFinding.confidence == heuristic` 时不能自动产生 Block（Goose 原则）
5. `SecurityFinding` 不自动产生 `PermissionDecision`（Goose 原则）

---

## 3. Crate 矩阵

### 3.1 KEEP — 直接采用，不修改

成熟且与 assurance 无关的 Grok 组件，作为 orz 的子 crate 直接使用：

| Crate | 功能 |
|-------|------|
| `orz-tools-api` | 工具接口定义 |
| `orz-workspace-client` | workspace 客户端 |
| `orz-workspace-types` | workspace 类型定义 |
| `orz-sandbox` | OS sandbox 隔离 |
| `orz-mcp` | Model Context Protocol |
| `orz-config-types` | 配置类型定义 |
| `orz-http` | HTTP 客户端基础设施 |
| `orz-markdown` | 终端 markdown 渲染 |
| `orz-markdown-core` | markdown 渲染核心 |
| `orz-paths` | 路径工具 |
| `orz-env` | 环境变量 |
| `orz-shared` | 共享工具 |
| `orz-version` | 版本信息 |
| `orz-agent` | agent 基础类型 |
| `xai-agent-lifecycle` | agent 生命周期 |
| `xai-file-utils` | 文件工具 |
| `xai-fast-worktree` | 快速 worktree |
| `xai-fsnotify` | 文件系统通知 |
| `xai-gix-status` | git status |
| `xai-hooks-plugins-types` | hook 插件类型 |
| `xai-token-estimation` | token 估算 |
| `xai-tty-utils` | TTY 工具 |
| `xai-system-power` | 系统电源 |
| `xai-prompt-queue` | prompt 队列 |
| `ptyctl` / `ptyctl-cli` | PTY 控制 |
| `xai-ratatui-inline` / `xai-ratatui-textarea` | TUI 组件 |
| `xai-tracing-macros` / `xai-crash-handler` | 诊断基础设施 |
| `xai-workflow` / `xai-hunk-tracker` | 工作流 |
| `xai-grok-pager` / `pager-render` / `pager-minimal` / `pager-bin` | Grok TUI (兜底 UI) |

### 3.2 MODIFY — 保留骨架，精准注入

| Crate | 修改位置 | 注入 ID |
|-------|---------|---------|
| `orz-shell` | `acp_agent.rs`: `new_session()`, `prompt()`, session close | IP4a-c |
| `orz-tools` | `bridge.rs`: `ToolBridge::call()` 前后 hook | IP3a-c |
| `orz-workspace` | 文件修改工具 hook | IP5 |
| `orz-hooks` | 标记 observe-only，hard gate 执行点 | IP6 (不变量) |
| `orz-config` | 移除 remote-settings 引用 | — |
| `orz-models` | 替换默认模型列表为 orz profiles (DeepSeek) | — |
| `orz-memory` | 添加 `memory.enabled` 配置开关（默认 false） | — |

### 3.3 REPLACE — 完全替换为自研实现

| Grok 原组件 | 代码量 | 替换为 | 说明 |
|------------|--------|--------|------|
| `orz-sampler` | ~10,000 行 | **Agent Loop Controller** (~1,550 行) | orz-loop 独立 crate |
| `orz-chat-state` | ~14,000 行 | **Blackboard** (~300 行) | orz-loop 内部模块 |
| `orz-telemetry` | ~13,600 行 | **hard no-op stub** | 编译时不初始化，不靠配置开关。参考 `xai-mixpanel` stub 模式 |
| `orz-auth` | ~400 行 | **no-op stub** | 本地 only，无云端认证 |
| `orz-plugin-marketplace` | ~5,600 行 | **no-op stub** | 不需要插件市场 |
| `orz-announcements` | ~400 行 | **no-op stub** | 不需要公告 |
| `orz-subagent-resolution` | ~3,000 行 | **Retrieval Subagents** | orz-loop 内部模块 |
| `xai-sqlite-journal` | — | **orz-assurance::journal** (JSONL) | hash-chained JSONL，非 SQLite |
| `xai-grok-secrets` | ~500 行 | **orz-assurance::credential** | Windows Credential Manager |
| `xai-mixpanel` | 已 stub | 保持 no-op stub | — |

### 3.4 ADD — 全新实现

| Crate | 来源 | 内容 |
|-------|------|------|
| `orz-bin` | 新 | composition root，产出 `orz` 二进制 |
| `orz-assurance` | Python spec → Rust | gates (ipg/tool_availability/source_visibility/adapter) + orientation (checkpoint/stagnation/sufficiency/trigger) + journal (event/chain/recorder/verifier) + session (trust/envelope/lifecycle/snapshot) + sandbox::job_object + credential + permit |
| `orz-tui` | Python `assurance/tui/` → Rust | assurance workbench TUI：gate 状态面板、journal 投影、orientation checkpoint 展示 |
| `orz-loop` | 新（自研） | Agent Loop Controller + MechanicalRelay + Blackboard + PromptBuilder + ModelGateway + ToolDispatcher + OrientationMonitor + Pro/Flash agents + Retrieval Subagents |

### 3.5 REMOVE — 完全删除

| Crate | 处理 |
|-------|------|
| `orz-update` | 删除（本地版本管理） |
| `orz-voice` | 删除（不需要语音） |
| `orz-mermaid` | 删除（不需要 Mermaid 渲染） |
| `orz-codebase-graph` | 删除（不需要代码图谱） |
| `orz-pager-pty-harness` | 删除（TUI PTY 测试） |
| `orz-test-support` | 合并到 dev-dependencies |
| `orz-shell-base` | 合并到 orz-shell |
| `orz-shell-session-support` | 合并到 orz-assurance::session |

---

## 4. 新的 Crate：orz-loop

### 4.1 决策

Agent Loop Controller 作为独立 crate `orz-loop`（`crates/orz-loop/`），而非合入 orz-shell。

**理由**：
1. ~1,550 行纯自研代码，与 Grok 代码零耦合——独立 crate 使边界清晰
2. 依赖 `orz-assurance` (gates/journal) + DeepSeek SDK + orz-shell (仅 ACP types)
3. `orz-shell` 简化：只保留 ACP 协议入口（`acp_agent.rs`），内部 dispatch 到 `orz-loop`
4. Grok 上游 rebase 时，`orz-loop` 完全不受影响

### 4.2 模块结构

```
crates/orz-loop/
├── Cargo.toml
└── src/
    ├── lib.rs                    # 模块注册 + 公共导出
    ├── controller.rs             # Agent Loop Controller 主 while 循环
    ├── prompt_builder.rs         # system prompt + assurance blocks
    ├── model_gateway.rs          # DeepSeek API → thinking:disabled + 流式响应
    ├── tool_dispatcher.rs        # 透传 orz-tools + IPG 注入
    ├── orientation_monitor.rs    # OrientationTrigger + cooldown 管理
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
orz-shell = { path = "../orz-shell" }        # ACP types only
orz-tools = { path = "../orz-tools" }         # tool dispatch
orz-tools-api = { path = "../orz-tools-api" }
orz-config = { path = "../orz-config" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
uuid = { version = "1", features = ["v4"] }
thiserror = "2"
tracing = "0.1"
```

### 4.4 orz-shell 的修改

orz-shell 不再包含 agent loop。它只负责 ACP 协议处理，然后委托给 orz-loop：

```rust
// orz-shell/src/agent/mvp_agent/acp_agent.rs
use orz_loop::AgentLoopController;

pub struct MvpAgent {
    loop_controller: AgentLoopController,
    config: OrzConfig,
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
        journal.record(RunEvent::run_preflight(&run_manifest, &trust_receipt))?;
        // ── END ORZ-INJECTION ──

        // 原有 new_session 逻辑...
    }

    async fn prompt(&self, arguments: acp::PromptRequest)
        -> Result<acp::PromptResponse, acp::Error>
    {
        // ── ORZ-INJECTION: turn-lifecycle (IP4b) ──
        self.loop_controller.next_turn();
        // ── END ORZ-INJECTION ──

        // 委托给 orz-loop
        let result = self.loop_controller.handle_prompt(arguments).await?;

        // ── ORZ-INJECTION: session-close (IP4c) ──
        // (在 session 关闭路径中)
        journal.record(RunEvent::run_finished(...));
        journal.seal()?;
        let receipt = journal.build_receipt()?;
        // ── END ORZ-INJECTION ──

        Ok(result)
    }
}
```

---

## 5. 解决已知矛盾

下表来自 Post-Plan A Review §2.4，每项均有明确裁决：

| 矛盾 | 裁决 | 依据 |
|------|------|------|
| Grok TUI 去留 | **保留为兜底**（双二进制架构） | 实现设计 v0.2 的方案，设计语言 v0.3 需修正 |
| Snapshot 归属 Phase | **Phase 4** | 实现设计 v0.2 / Agent Loop Redesign §9 |
| IP1 锚定矛盾 | **ModelGateway 内置**，不锚定 sampler | sampler 已被 Agent Loop Controller 替换 |
| Crate 命名 | 统一 **`orz-*`**，Grok TUI crates 保留 `xai-grok-*` 原名 | 与已执行代码一致 |
| GateDecision 命名 | 统一 **`Pass / Warn / Block / Defer / NotApplicable`** | 实现设计 v0.2+ |
| 文件名 vs 内容版本 | **本文档作为统一真相源**，源文档降级为参考 | 本文定义最终版本 |

### 关于 design language v0.3 §2 的修正

v0.3 §2 中部分定级与最终设计不一致：

| v0.3 定级 | 修正为 | 理由 |
|-----------|--------|------|
| Grok TUI → REPLACE | **KEEP** (兜底 UI) | 双二进制架构，grok 二进制保留 |
| Snapshot → Phase 2 | **Phase 4** | 实现设计 v0.2 的编排 |
| Crate 命名 `xai-grok-*` | **统一 `orz-*`** | 与已执行代码一致 |

---

## 6. 修正 4 个伪代码 Bug

来自 Post-Plan A Review §2.3：

| # | Bug | 影响 | 修正 |
|---|-----|------|------|
| 1 | `SnapshotStore` sync 方法内用 `.await` — 无法编译 | 编译错误 | 所有方法改为 `async fn`：`async fn track()`, `async fn patch()`, `async fn restore()`, `async fn revert()` |
| 2 | `JournalRecorder::record()` 用 `try_send` — channel full 时静默丢事件 | hash chain 断裂 | 改为 `send().await` 阻塞发送。`record()` 改为 `async fn`；无法容忍背压时返回 `Result` 而非静默丢弃 |
| 3 | `orphan_events` 维度永无法填充（`RunEvent` 无 `turn_id`） | 死代码 | 从 `JournalInvariants` 中移除 `orphan_events`；或给 `RunEvent` 增加 `turn_id: Option<u64>` 字段。**裁决：移除 `orphan_events`**，三项不变量足够（dup/timestamp/hash） |
| 4 | verifier 不重算 `event_sha256` — payload 篡改无法检测 | 安全缺口 | 增加 `compute_event_sha256()` 步骤：对每个 event 排除 `event_sha256` 字段后重算 SHA-256，与存储值对比。不匹配时计入 `hash_chain_breaks` |

修正后的 `JournalInvariants`（三项不变量）：

```rust
#[derive(Debug, Default)]
pub struct JournalInvariants {
    pub duplicate_event_ids: Vec<String>,
    pub timestamp_violations: Vec<u64>,
    pub hash_chain_breaks: Vec<u64>,  // 含 prev_sha256 mismatch + event_sha256 mismatch
}

impl JournalInvariants {
    pub fn check(events: &[RunEvent]) -> Self {
        let mut invariants = Self::default();
        let mut seen_ids = std::collections::HashSet::new();

        for (i, event) in events.iter().enumerate() {
            // 1. 重复 ID
            if !seen_ids.insert(&event.event_id) {
                invariants.duplicate_event_ids.push(event.event_id.clone());
            }
            if i > 0 {
                // 2. 时间戳线性
                if event.timestamp < events[i - 1].timestamp {
                    invariants.timestamp_violations.push(i as u64);
                }
                // 3. Hash chain continuity
                //    a) previous_event_sha256 必须匹配前一个 event 的 event_sha256
                if event.previous_event_sha256.as_deref()
                    != Some(&events[i - 1].event_sha256)
                {
                    invariants.hash_chain_breaks.push(i as u64);
                }
            }
            // 4. event_sha256 独立重算（Bug #4 修正）
            let computed = compute_event_sha256(event);
            if computed != event.event_sha256 {
                invariants.hash_chain_breaks.push(i as u64);
            }
        }
        invariants
    }

    pub fn is_clean(&self) -> bool { /* ... */ }
}

fn compute_event_sha256(event: &RunEvent) -> String {
    // 序列化 event 的全部字段（排除 event_sha256 自身），计算 SHA-256
    // ...
}
```

修正后的 `JournalRecorder`（阻塞 send）：

```rust
impl JournalRecorder {
    /// 记录一个事件。异步——channel full 时阻塞等待，不静默丢弃。
    pub async fn record(&self, event: RunEvent) -> io::Result<()> {
        self.tx.send(JournalCmd::AddEvents(vec![event]))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "journal writer task terminated"))
    }
}
```

---

## 7. 实施阶段

### Phase 1: Scaffold
**目标**：`cargo build --release` 产出 `orz.exe` + `grok.exe`

1. Fork Grok Build `500129c7`
2. 删除 §3.5 REMOVE crates（git rm + workspace Cargo.toml 移除）
3. 创建 ADD crates 空骨架：`orz-bin/`, `orz-assurance/`, `orz-tui/`, `orz-loop/`
4. 替换 REPLACE crates 为 no-op stub：telemetry, auth, plugin-marketplace, announcements
5. 重命名 kept crates (`xai-grok-*` → `orz-*`)，Grok TUI crates 保留原名
6. 修改 `orz-bin/` 和 `xai-grok-pager-bin/`：裁剪 telemetry/auth/update 引用
7. 更新 workspace `Cargo.toml`（双二进制 + 全部 crate members）
8. 添加 `memory.enabled = false` 默认配置
9. **验证**：`./target/release/orz --version`, `./target/release/grok --version`

### Phase 2: Journal + Transport + 单 Agent Loop
**目标**：`orz -p "hello"` → 有效 `events.jsonl`，hash chain 连续

1. 实现 `orz-assurance::journal::event` (RunEvent 类型)
2. 实现 `orz-assurance::journal::chain` (SHA-256 hash chain)
3. 实现 `orz-assurance::journal::recorder` (Codex 模式 async channel，**阻塞 send**)
4. 实现 `orz-assurance::journal::verifier` (三项不变量 + event_sha256 重算)
5. 实现 `orz-loop` **单 Agent 版本**（先不上 Pro/Flash 双模型）：
   - `PromptBuilder`：system prompt + `[TOOL_AVAILABILITY]` (IP2a)
   - `ModelGateway`：DeepSeek API + `thinking: disabled` 内置 (IP1)
   - `OrientationMonitor`：事件驱动触发 + cooldown (IP2b)
   - `ToolDispatcher`：透传 orz-tools
   - 主 `while` 循环（单 Agent 路径）
6. IP4a/IP4c：session lifecycle journal events @ ACP entry（orz-shell `acp_agent.rs`）
7. IP3a-c：tool dispatch 注入 @ orz-tools `bridge.rs`
8. **验证**：`orz -p "hello"` → 有效 `events.jsonl`，hash chain 连续，Python `canonical_cli.py` 可独立验证

### Phase 3: 双 Agent + Blackboard + Gates
**目标**：Pro + Flash + 检索子代理完整协作

1. 实现 `Blackboard`（PlanSection / ExecSection / InternalRet / ExternalRet / GateLog）
2. 实现 `MechanicalRelay`（纯 function.name 路由表）
3. Pro/Flash 双 Agent（对等配置，角色 Prompt 不同）
4. Flash 授权流程（请求 → Pro 审查 → 授权/拒绝）
5. 实现 `orz-assurance::gates`（ipg, tool_availability, source_visibility, adapter）
6. 实现 `orz-assurance::orientation`（checkpoint, stagnation, sufficiency）
7. Retrieval Subagents（内部 ProjectDoc + 外部 WebSearch 处理）
8. IP2c：`INFO_SUFFICIENCY_CHECK` @ PromptBuilder
9. IP6：hard gate enforcement（架构不变量）
10. **验证**：完整 Pro+Flash+子代理+gate 链

### Phase 4: TUI + Snapshot + Polish
**目标**：完整 orz 产品

1. 移植 Python `assurance/tui/` → `orz-tui`（assurance workbench）
2. 实现 `orz-assurance::session::snapshot`（OpenCode 模式 shadow Git，**async fn**）
3. IP5：pre-mutation snapshot @ orz-workspace
4. 实现 `orz-assurance::session`（trust, envelope, lifecycle）
5. 实现 `orz-assurance::sandbox::job_object`
6. 实现 `orz-assurance::credential` (Windows Credential Manager)
7. 实现 `orz-assurance::permit` (one-shot action permit)
8. Grok TUI 兜底裁剪（移除 voice/update/mermaid/announcements 引用）
9. Telemetry **硬 no-op**：编译时 `#[cfg(feature = "telemetry")]` gate，默认 off
10. Python 项目（`D:\CLI`）标记 `reference-spec`
11. **验证**：orz 全功能通过 Python conformance suite

---

## 8. Telemetry 硬禁用 具体方案

Post-Plan A Review 发现 `orz-shell/src/agent/init.rs:215` 在共享初始化路径中调用
`orz_telemetry::client::init(...)`，且仍使用旧 `GROK_TELEMETRY_ENABLED` 环境变量前缀。

**修正**：

1. `orz-telemetry` 全部公共 API 改为 no-op（参考 `xai-mixpanel` stub 模式）
2. `orz-shell/src/agent/init.rs` 中的 `orz_telemetry::client::init(...)` 替换为
   `// telemetry disabled — see orz-assurance design` 注释
3. 环境变量 `GROK_TELEMETRY_ENABLED` 删除，替换为 `ORZ_`
4. Compile-time guard:
   ```toml
   # orz-telemetry/Cargo.toml
   [features]
   default = []
   telemetry = []  # 仅在显式 opt-in 时启用
   ```
   默认不启用 `telemetry` feature，所有构建均无遥测。

---

## 9. 参考

### 被整合的源文档
- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.1.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.1.md) — fork 设计 v0.1
- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.2.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.2.md) — 拆解 Grok + 混搭
- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md) — 五源融合 + orz 命名
- [`FORK_IMPLEMENTATION_DESIGN_v0.1.md`](FORK_IMPLEMENTATION_DESIGN_v0.1.md) — fork 实现设计 (内容 v0.2)
- [`AGENT_LOOP_REDESIGN_v0.1.md`](AGENT_LOOP_REDESIGN_v0.1.md) — Agent Loop 重设计

### 审查与分析
- [`PHASE2_POST_PLANA_REVIEW_v0.1.md`](PHASE2_POST_PLANA_REVIEW_v0.1.md) — Plan A 后审查
- [`PHASE2_COMPILATION_STATUS_v0.2.md`](PHASE2_COMPILATION_STATUS_v0.2.md) — 编译修复记录
- [`IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md`](IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md) — 偏差分析

### 外部参考
- Codex CLI: `codex-rs/rollout/src/recorder.rs` (Apache-2.0)
- Gemini CLI: `packages/core/src/context/utils/invariantChecker.ts` (Apache-2.0)
- OpenCode: `packages/opencode/src/snapshot/index.ts` (MIT)
- Goose: `crates/goose/src/security/security_inspector.rs` (Apache-2.0)
- Grok Build: `xai-grok-shell`, `xai-grok-tools`, `xai-grok-workspace` (Apache-2.0)
- D 项目存档：`G:\我的云端硬盘\VSCode_Copilot_Archives\` — 黑板/蜂群传导参考

### ADR
- ADR-0001: evidence-constrained kernel
- ADR-0003: runtime-neutral assurance kernel
