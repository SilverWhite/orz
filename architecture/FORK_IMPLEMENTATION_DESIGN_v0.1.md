# Fork 实现设计 v0.2

状态：2026-08-02。基于对 Grok Build `500129c7` 63-crate 源码树的直接审查，
确定每一个 crate 的去留、每一个注入点的精确位置和代码。所有 assurance 逻辑
编译进二进制内部，不存在外部进程、sidecar wrapper 或 HTTP proxy。

**v0.2 关键变更**（从 v0.1）：
- **双二进制架构**：`orz`（自研 TUI）+ `grok`（原生 Grok TUI 兜底），共享同一底层
- **记忆系统保留**：默认关闭，用户显式启用后本地运行

## 0. 前置：源码获取与基线

```bash
git clone https://github.com/xai-org/grok-build orz
cd orz
git checkout 500129c714ad1b10e6095481f4a8387a2ec52649
# SOURCE_REV = 6372e41d828b8a6ee82c29e01a69e27ec895cca9
```

以此为基线，下列所有操作在同一个 Git 历史中完成。

## 1. Crate 去留矩阵（63 → ~35）

### 1.1 完整删除（18 crates）

直接 `git rm -r crates/codegen/<name>`，从 workspace `Cargo.toml` 中移除：

| Crate | 理由 |
|-------|------|
| `xai-grok-telemetry` | 遥测 → 删除 |
| `xai-grok-auth` | 云端认证 → 删除 |
| `xai-grok-update` | 更新通道 → 删除 |
| `xai-grok-voice` | 语音功能 → 删除 |
| `xai-grok-announcements` | 公告 → 删除 |
| `xai-grok-plugin-marketplace` | 插件市场 → 删除 |
| `xai-mixpanel` | 分析 → 删除 |
| `xai-sqlite-journal` | SQLite journal → 替换为 `orz-assurance::journal` (JSONL) |
| `xai-grok-subagent-resolution` | 子代理解析 → orz 不需要 |
| `xai-grok-sampler` | 采样器 → 修改后合并到 shell（见 §3.1） |
| `xai-grok-codebase-graph` | 代码图谱 → orz 不需要 |
| `xai-grok-secrets` | 密钥存储 → 替换为 `orz-assurance::credential` (Windows Credential Manager) |
| `xai-grok-pager-pty-harness` | TUI PTY 测试 → 不需要 |
| `xai-grok-test-support` | 合并到 dev-dependencies |
| `xai-grok-shell-session-support` | 合并到 `orz-assurance::session` |
| `xai-grok-shell-base` | 合并到 shell 或 orz-assurance |
| `xai-chat-state` | 聊天状态 → orz 用 journal 管理状态 |
| `xai-grok-mermaid` | Mermaid 渲染 → orz 不需要 |

### 1.2 保留但修改（19 crates）

| Crate | 操作 | 修改文件 |
|-------|------|---------|
| `xai-grok-shell` | **重命名** `orz-shell`，注入 6 个点 | 见 §3 |
| `xai-grok-tools` | **保留**，注入 tool dispatch | `bridge.rs` |
| `xai-grok-tools-api` | **保留**，不修改 | — |
| `xai-grok-workspace` | **保留**，注入 snapshot | 见 §3.5 |
| `xai-grok-workspace-client` | **保留**，不修改 | — |
| `xai-grok-workspace-types` | **保留**，不修改 | — |
| `xai-grok-sandbox` | **保留**，不修改 | — |
| `xai-grok-mcp` | **保留**，不修改 | — |
| `xai-grok-config` | **保留**，移除 remote-settings | `config/` |
| `xai-grok-config-types` | **保留**，不修改 | — |
| `xai-grok-http` | **保留**，不修改（只提供 HTTP client infra） | — |
| `xai-grok-models` | **保留**，替换默认模型列表为 orz profile | `default_models.rs` |
| `xai-grok-hooks` | **保留**，标记 observe-only（不承载 hard gate） | `lib.rs` |
| `xai-acp-lib` | **保留**，不修改 | — |
| `xai-grok-pager` | **保留**，作为 Grok 原生 TUI（兜底 UI） | 仅命名/品牌文字替换 |
| `xai-grok-pager-render` | **保留**，Grok TUI 渲染层 | — |
| `xai-grok-pager-minimal` | **保留**，Grok TUI 最小化启动 | — |
| `xai-grok-pager-bin` | **保留**，产出 `grok` 二进制（兜底 UI 入口） | 裁剪 telemetry/auth/update 引用 |
| `xai-grok-memory` | **保留**，默认关闭，用户显式启用后本地运行 | `config.rs`：添加 `memory.enabled` 开关 |

### 1.3 保留不修改（~14 crates）

`xai-grok-markdown`, `xai-grok-markdown-core`, `xai-grok-paths`, `xai-grok-env`,
`xai-grok-shared`, `xai-grok-version`, `xai-grok-agent`, `xai-agent-lifecycle`,
`xai-file-utils`, `xai-fast-worktree`, `xai-fsnotify`, `xai-gix-status`,
`xai-hooks-plugins-types`, `xai-token-estimation`, `xai-tty-utils`, `xai-system-power`,
`xai-prompt-queue`, `ptyctl`, `ptyctl-cli`, `xai-ratatui-inline`, `xai-ratatui-textarea`,
`xai-tracing-macros`, `xai-crash-handler`, `xai-workflow`, `xai-hunk-tracker`

### 1.4 新增（3 crates）

| Crate | 来源 |
|-------|------|
| `orz-bin` | 新 composition root，产出 `orz` 二进制（自研 TUI 入口） |
| `orz-assurance` | 从 Python `assurance/` 移植（gates, orientation, journal, session, sandbox, credential, permit） |
| `orz-tui` | 从 Python `assurance/tui/` 移植（assurance workbench TUI） |

### 1.5 双二进制架构

```
                     orz-assurance (gates, journal, orientation, snapshot, ...)
                              │
                              │ 编译时链接（不是外部 sidecar）
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                    │
    orz-shell             orz-tools           orz-workspace
    (ACP, agent loop)     (tool dispatch)     (filesystem, VCS)
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
         orz-bin                          xai-grok-pager-bin
         (orz TUI 入口)                   (Grok 原生 TUI 入口)
              │                               │
         orz-tui                         xai-grok-pager
         (assurance workbench)            (Grok 原生 TUI)
              │                               │
              ▼                               ▼
            orz                             grok
       "主 UI，日常使用"                "兜底 UI，紧急备用"
```

**两个二进制共享完全相同的底层**：

- 同一个 `orz-shell`（含所有 assurance 注入）
- 同一个 `orz-tools`（含 IPG + orientation counters）
- 同一个 `orz-workspace`（含 snapshot）
- 同一个 `orz-assurance`（gates, journal, session, sandbox, credential, permit）

**区别仅在 UI 层**：

| | `orz` | `grok` |
|---|---|---|
| TUI | `orz-tui`（自研 assurance workbench） | `xai-grok-pager`（Grok 原生 TUI） |
| Gate 状态可见性 | ✅ 面板展示 gate/joural/orientation | ❌ 不可见，但 gate 仍在运行 |
| Journal 投影 | ✅ TUI 实时事件流 | ❌ 不可见，但 journal 仍在写入 |
| Assurance 行为 | 完全相同 | 完全相同 |
| 用途 | 日常主 UI | 兜底——orz TUI 出问题时紧急使用 |

**设计意图**：

1. `orz` TUI 是主界面——展示 assurance 状态、gate 决策、journal 投影、orientation checkpoint。
2. `grok` 是纯兜底——如果 orz TUI 有 bug 或渲染问题，用户可以随时 `grok` 回到熟悉的原生界面。
3. 兜底**不是降级**——使用 `grok` 时 assurance 层完整运行，只是不可见。journal 照写，gate 照跑。
4. 这直接实现了 INTEGRATED_AGENT_ASSURANCE_DESIGN 中的 `ui_strategy.primary = orz_tui / fallback = grok_native_ui`。

**实现方式**：

```toml
# Cargo.toml (workspace)
[[bin]]
name = "orz"
path = "crates/orz-bin/src/main.rs"

[[bin]]
name = "grok"
path = "crates/codegen/xai-grok-pager-bin/src/main.rs"
# 同上但裁剪了 telemetry/auth/update 引用
# 链接相同的 orz-shell, orz-tools, orz-assurance
```

### 1.6 记忆系统：保留但默认关闭

```toml
# 默认配置
[memory]
enabled = false          # 默认关闭
storage = "local"        # 仅本地（无云端同步）
path = "~/.orz/memory"   # 存储路径
```

- 评测/holdout 场景强制 `enabled = false`
- 用户显式 `orz config set memory.enabled true` 后启用
- 记忆系统代码保留但启动时检查开关
- 记忆内容不进入 journal（只记录 `memory_enabled: true/false` 到 run manifest）

## 2. 重命名：全局 gsa/grok → orz

### 2.1 二进制名

```
xai-grok-pager → orz
grok → orz（符号链接或别名）
```

### 2.2 Crate 重命名

```toml
# 在各自 Cargo.toml 中:
xai-grok-shell → orz-shell
xai-grok-tools → orz-tools
xai-grok-workspace → orz-workspace
xai-grok-sandbox → orz-sandbox
xai-grok-mcp → orz-mcp
xai-grok-config → orz-config
xai-grok-http → orz-http
xai-grok-models → orz-models
xai-grok-hooks → orz-hooks
xai-grok-markdown → orz-markdown
# ... 其余类推
```

### 2.3 目录重命名

```
~/.grok/ → ~/.orz/
GROK_WORKER_THREADS → ORZ_WORKER_THREADS
# 所有环境变量前缀
```

### 2.4 代码中的重命名

```rust
// 全局替换:
xai_grok_ → orz_
XAI_GROK_ → ORZ_
Grok → Orz (在面向用户的字符串中)
grok → orz (在路径/文件名中)
```

## 3. 注入点：精确位置与代码

### 3.1 注入点 1: HTTP 传输 — thinking disabled

**文件**: `orz-models` + `orz-shell` 中构建 Chat Completions body 的位置

**现状**: `deepseek_thinking_proxy.py`（外部 HTTP 代理）注入 `thinking: disabled`

**源码定位**: 实际 Chat Completions 请求构建可能在 `orz-shell/src/sampling/` 或
session actor 中（代码路径：`MvpAgent::prompt()` → session actor →
sampler → HTTP request）。需在 clone 后确认精确文件，但注入逻辑确定：

```rust
// 在序列化 Chat Completions 请求 body 之前插入:
// GSA-INJECTION: thinking-disabled (injection point 1)
// Replaces scripts/deepseek_thinking_proxy.py
if config.model_provider == ModelProvider::DeepSeek {
    body["thinking"] = json!({"type": "disabled"});
}
```

**验证**: 删除 `scripts/deepseek_thinking_proxy.py` 后，`orz -p "test"` 发送的请求中
body 包含 `"thinking":{"type":"disabled"}`。

### 3.2 注入点 2: Prompt 管线 — assurance context blocks

**文件**: `orz-shell/src/agent/mvp_agent/mod.rs`

**现有函数**: `build_spawn_system_prompt` (已定位)

**注入内容**: 在 system prompt 组装之后、发送给模型之前：

```rust
// GSA-INJECTION: assurance-prompt-blocks (injection point 2a)
// Tool availability context — session 开始时注入一次
use orz_assurance::gates::tool_availability;
use orz_assurance::orientation::{checkpoint, sufficiency, trigger};

// 在 session spawn 时:
let tool_avail_ctx = tool_availability::build_context_block(available_tools)?;
system_prompt.push_str(&tool_avail_ctx);

// GSA-INJECTION: orientation-checkpoint (injection point 2b)
// 动作间隙注入，事件驱动:
if orientation_trigger.should_fire() {
    let ckpt = checkpoint::build(
        task_id, orientation_trigger.conversation_turn,
        tool_availability_sha256, checklist_context,
    )?;
    // 注入到下一轮模型调用的 system prompt 之前
    pending_system_prompt_prefix.push_str(&ckpt.message_block);
    orientation_trigger.reset_cooldown();
}

// GSA-INJECTION: info-sufficiency-check (injection point 2c)
// 检索工具完成后，正式回复前:
if pending_sufficiency_check {
    let check = sufficiency::build(
        sources_count, categories, full_text_visible_count)?;
    pending_system_prompt_prefix.push_str(&check);
    pending_sufficiency_check = false;
}
```

### 3.3 注入点 3: 工具调度 — IPG + action auth

**文件**: `orz-tools/src/bridge.rs`

**现有函数**: `ToolBridge::call(client_function_name, client_params, tool_call_id)`

**注入代码**:

```rust
impl ToolBridge {
    pub fn call(
        &self,
        client_function_name: &str,
        client_params: serde_json::Value,
        tool_call_id: String,
    ) -> Result<ToolRunResult, ToolError> {
        // ── GSA-INJECTION: ipg-gate (injection point 3a) ──
        // 工具执行前：指令来源检查
        let ipg_ctx = orz_assurance::gates::ipg::build_context(
            &client_function_name, &client_params, &tool_call_id)?;
        let ipg_result = orz_assurance::gates::ipg::evaluate(&ipg_ctx)?;
        match ipg_result.decision {
            GateDecision::Block => {
                // 记录到 journal
                journal_recorder.record(RunEvent::gate_blocked(
                    "instruction_provenance_gate", &ipg_result));
                return Err(ToolError::GateBlocked(ipg_result));
            }
            GateDecision::Warn => {
                journal_recorder.record(RunEvent::gate_warned(
                    "instruction_provenance_gate", &ipg_result));
            }
            _ => {}
        }

        // ── 原有 dispatch 逻辑 ──
        let result = self.registry.call(client_function_name, client_params, None)?;

        // ── GSA-INJECTION: orientation-counters (injection point 3b) ──
        // 工具执行后：更新 orientation 计数器
        orientation_trigger.record_action();
        orientation_trigger.record_tool_type(client_function_name);

        // ── GSA-INJECTION: sufficiency-trigger (injection point 3c) ──
        // 检索工具完成后标记需要信息收集确认
        if is_retrieval_tool(client_function_name) {
            trigger_pending_sufficiency_check();
        }

        Ok(result)
    }
}
```

### 3.4 注入点 4: 会话生命周期 — journal events

**文件**: `orz-shell/src/agent/mvp_agent/acp_agent.rs`

**现有函数**: `MvpAgent::new_session()`, `MvpAgent::prompt()` （`impl acp::Agent for MvpAgent`）

**注入代码**:

```rust
impl acp::Agent for MvpAgent {
    async fn new_session(&self, arguments: acp::NewSessionRequest)
        -> Result<acp::NewSessionResponse, acp::Error>
    {
        // ── GSA-INJECTION: session-lifecycle (injection point 4a) ──
        // 1. Workspace trust 验证
        let trust_receipt = orz_assurance::session::trust::verify(
            &arguments.cwd, &self.config)?;
        if !trust_receipt.trusted {
            return Err(acp::Error::custom("workspace trust denied"));
        }

        // 2. 创建 journal recorder (Codex 模式 async channel)
        let journal = orz_assurance::journal::recorder::JournalRecorder::new(
            &run_root.join("events.jsonl"))?;

        // 3. Security envelope 初始化
        let envelope = orz_assurance::session::envelope::SecurityEnvelope::new(
            &run_manifest)?;

        // 4. 写入 run_preflight event
        journal.record(RunEvent::run_preflight(
            &run_manifest, &trust_receipt))?;

        // ── 原有 new_session 逻辑 ──
        let response = /* ... existing code ... */;

        Ok(response)
    }

    async fn prompt(&self, mut arguments: acp::PromptRequest)
        -> Result<acp::PromptResponse, acp::Error>
    {
        // ── GSA-INJECTION: prompt-lifecycle (injection point 4b) ──
        // 轮次计数
        orientation_trigger.next_turn();

        // ── 原有 prompt 逻辑 ──
        // ... dispatch to session actor ...

        // ── GSA-INJECTION: session-close (injection point 4c) ──
        // 在 session 关闭时（prompt 完成或错误）:
        // journal.record(RunEvent::run_finished(...));
        // journal.seal()?;
        // let receipt = journal.build_receipt()?;
        Ok(response)
    }
}
```

### 3.5 注入点 5: Snapshot（OpenCode 模式）

**文件**: `orz-workspace` 中的工具执行前 hook

**注入代码**:

```rust
// GSA-INJECTION: pre-mutation-snapshot (injection point 5)
// 在任何会修改文件的工具执行前创建快照:
use orz_assurance::session::snapshot::SnapshotStore;

let snapshot_store = SnapshotStore::init(project_id, worktree)?;

// 工具执行前:
if tool_modifies_files(&tool_name) {
    let snapshot_hash = snapshot_store.track()?;
    journal.record(RunEvent::snapshot_created(&snapshot_hash));
}

// 恢复时（用户显式请求），记录到 journal:
// journal.record(RunEvent::snapshot_restored(snapshot_hash, scope));
// 恢复成功不改变原 permission 决策
```

### 3.6 注入点 6: Permission → hard gate 不可绕过

**文件**: `orz-hooks` 的 hook 执行路径 + permission 决策点

**注入约束**（编译时保证，不是运行时 hook）:

```rust
// GSA-INJECTION: hard-gate-enforcement (injection point 6)
// 在 permission 最终决策后:
//
// INVARIANT: hook 返回值只能从 allow 降级到 deny/ask，不能从 deny 升级到 allow
// INVARIANT: hard gate decision (Block/Defer) 不能被 hook 或用户偏好覆盖
// INVARIANT: 所有 permission decision 写入 journal event
```

## 4. orz-assurance 模块级设计

### 4.1 Cargo.toml

```toml
[package]
name = "orz-assurance"
version = "0.1.0"
edition = "2024"
license = "Apache-2.0"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
hex = "0.4"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
tokio = { version = "1", features = ["sync", "fs", "io-util"] }
git2 = { version = "0.20", default-features = false, features = ["vendored-libgit2"] }
windows = { version = "0.58", features = ["Security", "System", "Win32"] }
jsonschema = "0.18"
thiserror = "2"

[dev-dependencies]
tempfile = "3"
tokio-test = "0.4"
```

### 4.2 lib.rs — 模块注册

```rust
// orz-assurance/src/lib.rs

pub mod gates;
pub mod orientation;
pub mod journal;
pub mod session;
pub mod sandbox;
pub mod credential;
pub mod permit;

// 全局错误类型
#[derive(Debug, thiserror::Error)]
pub enum AssuranceError {
    #[error("gate blocked: {0:?}")]
    GateBlocked(GateDecision),
    #[error("journal error: {0}")]
    Journal(#[from] std::io::Error),
    #[error("hash chain broken at event {sequence}: expected {expected}, got {actual}")]
    HashChainBroken { sequence: u64, expected: String, actual: String },
    #[error("invariant violation: {0}")]
    InvariantViolation(String),
    #[error("schema validation: {0}")]
    SchemaValidation(String),
    #[error("credential: {0}")]
    Credential(String),
}

// 重新导出核心类型
pub use gates::{AssuranceGate, GateDecision};
pub use journal::event::{RunEvent, EventType, RedactionLevel};
pub use journal::recorder::JournalRecorder;
pub use journal::verifier::JournalInvariants;
pub use orientation::trigger::OrientationTrigger;
pub use session::envelope::SecurityEnvelope;
pub use session::trust::WorkspaceTrustReceipt;
```

### 4.3 gates/ — 门禁模块

```rust
// orz-assurance/src/gates/mod.rs

use serde::{Deserialize, Serialize};

/// 所有 gate 实现此 trait。
pub trait AssuranceGate {
    type Context;
    type Receipt: Serialize + for<'de> Deserialize<'de>;

    fn evaluate(&self, ctx: &Self::Context) -> Result<Self::Receipt, AssuranceError>;
    fn verify(receipt: &Self::Receipt, ctx: &Self::Context) -> Result<bool, AssuranceError>;
}

/// Gate 决策五值枚举（对齐 PROTOCOL_DRAFT §9）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateDecision {
    Pass,
    Warn { reason_codes: Vec<String> },
    Block { reason_codes: Vec<String> },
    Defer { missing: Vec<String> },
    NotApplicable,
}

pub mod ipg;                // instruction_provenance_gate.py → Rust
pub mod tool_availability;  // tool_availability_gate.py → Rust
pub mod source_visibility;  // source_visibility.py → Rust
pub mod adapter;            // adapter_gate.py → Rust
```

**ipg.rs 结构**（从 `instruction_provenance_gate.py` 移植）：

```rust
// orz-assurance/src/gates/ipg.rs

use super::{AssuranceGate, AssuranceError, GateDecision};
use serde::{Deserialize, Serialize};

pub struct InstructionProvenanceGate;

pub struct IpgContext {
    pub instruction_text: String,
    pub source_type: SourceType,  // user_input | project_rule | hook | plugin | mcp | ...
    pub injection_patterns: Vec<InjectionPattern>,
    pub instruction_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    UserInput,
    ProjectRule,
    Hook,
    Plugin,
    McpConfig,
    SkillDefinition,
    MemoryContent,
    ExternalRetrieved,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjectionPattern {
    DirectOverride,
    IndirectPersuasion,
    ClaimFabrication,
    BoundaryBlur,
    Clean,  // no injection detected
}

pub struct IpgReceipt {
    pub schema_version: String,
    pub receipt_kind: String,
    pub valid: bool,
    pub decision: GateDecision,
    pub instruction_sha256: String,
    pub source_type: SourceType,
    pub injection_patterns: Vec<InjectionPattern>,
    pub reason_codes: Vec<String>,
    pub checks: IpgChecks,
}

impl AssuranceGate for InstructionProvenanceGate {
    type Context = IpgContext;
    type Receipt = IpgReceipt;

    fn evaluate(&self, ctx: &Self::Context) -> Result<Self::Receipt, AssuranceError> {
        // 实现 instruction_provenance_gate.py 的 evaluate 逻辑:
        // 1. 检查 source_type 是否合法
        // 2. 扫描 injection_patterns
        // 3. 生成 decision (pass/warn/block/defer)
        // ...
    }

    fn verify(receipt: &Self::Receipt, ctx: &Self::Context) -> Result<bool, AssuranceError> {
        // 独立验证: 用 ctx 重建 expected receipt，逐字段比较
        // ...
    }
}
```

### 4.4 orientation/ — 中立问询模块

```rust
// orz-assurance/src/orientation/mod.rs

pub mod checkpoint;   // build_orientation_checkpoint, verify_orientation_response
pub mod stagnation;   // evaluate_runtime_stagnation_guard
pub mod sufficiency;  // INFO_SUFFICIENCY_CHECK (新)
pub mod trigger;      // OrientationTrigger (事件驱动)
```

**trigger.rs**（新设计，替换 `fixed_step_interval`）：

```rust
// orz-assurance/src/orientation/trigger.rs

use std::collections::HashSet;

/// 事件驱动的 orientation checkpoint 触发控制器。
///
/// 总是在动作间隙（工具执行完成、模型调用前）检查。
/// 轮次触发不受 cooldown 限制。
pub struct OrientationTrigger {
    pub turn_output_tokens: u64,
    pub turn_action_count: u64,
    pub distinct_tool_types: HashSet<String>,
    pub conversation_turn: u64,
    pub cooldown_remaining: u64,
    /// 标记下一轮模型调用前需要触发 INFO_SUFFICIENCY_CHECK
    pub pending_sufficiency_check: bool,
}

impl OrientationTrigger {
    pub fn new() -> Self {
        Self {
            turn_output_tokens: 0,
            turn_action_count: 0,
            distinct_tool_types: HashSet::new(),
            conversation_turn: 0,
            cooldown_remaining: 0,
            pending_sufficiency_check: false,
        }
    }

    /// 是否需要在下一个动作间隙注入 orientation checkpoint。
    pub fn should_fire(&self) -> bool {
        // 轮次触发 — 不受 cooldown 限制
        if self.conversation_turn > 0 && self.conversation_turn % 8 == 0 {
            return true;
        }
        // 硬门控触发 — 受 cooldown 限制
        if self.cooldown_remaining > 0 {
            return false;
        }
        self.turn_output_tokens >= 7000
            || self.turn_action_count >= 10
            || self.distinct_tool_types.len() >= 7
    }

    /// 记录一次工具执行完成。
    pub fn record_action(&mut self) {
        self.turn_action_count += 1;
        if self.cooldown_remaining > 0 {
            self.cooldown_remaining -= 1;
        }
    }

    /// 记录公开输出的 token 数。
    pub fn record_tokens(&mut self, count: u64) {
        self.turn_output_tokens += count;
    }

    /// 记录使用的工具类型（去重计数）。
    pub fn record_tool_type(&mut self, tool_id: &str) {
        self.distinct_tool_types.insert(tool_id.to_string());
    }

    /// 进入下一轮对话。
    pub fn next_turn(&mut self) {
        self.conversation_turn += 1;
        self.turn_output_tokens = 0;
        self.turn_action_count = 0;
        self.distinct_tool_types.clear();
    }

    /// 触发 orientation checkpoint 后重置 cooldown。
    pub fn reset_cooldown(&mut self) {
        self.cooldown_remaining = 4;
    }

    /// 标记需要在下一轮触发信息收集确认。
    pub fn trigger_sufficiency_check(&mut self) {
        self.pending_sufficiency_check = true;
    }
}
```

**sufficiency.rs**（新设计）：

```rust
// orz-assurance/src/orientation/sufficiency.rs

/// INFO_SUFFICIENCY_CHECK — 检索后、正式回复前注入。
/// 只陈述机械可验证的事实，不暗示质疑当前结论。
pub const INFO_SUFFICIENCY_BLOCK: &str = "\
[INFO_SUFFICIENCY_CHECK v0.1]
已获取来源: {source_count} 项
覆盖范围: {categories}
全文可见: {full_text_count}/{source_count}
当前信息是否足够回答用户问题？如不足，还需哪些信息？
[/INFO_SUFFICIENCY_CHECK]";

pub fn build(
    source_count: usize,
    categories: &[String],
    full_text_visible_count: usize,
) -> String {
    INFO_SUFFICIENCY_BLOCK
        .replace("{source_count}", &source_count.to_string())
        .replace("{categories}", &categories.join(", "))
        .replace("{full_text_count}", &full_text_visible_count.to_string())
}
```

### 4.5 journal/ — 事件日志模块

```rust
// orz-assurance/src/journal/mod.rs

pub mod event;      // RunEvent, EventType, RedactionLevel
pub mod recorder;   // JournalRecorder (Codex 模式 async channel)
pub mod chain;      // SHA-256 hash chain
pub mod verifier;   // JournalInvariants (Gemini 模式) + hash chain verification
```

**recorder.rs**（Codex 模式）：

```rust
// orz-assurance/src/journal/recorder.rs

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use super::event::RunEvent;

/// 基于 Codex `RolloutRecorder` 模式的异步 journal 写入器。
///
/// - 廉价 clone：所有 clone 共享同一 channel 和 writer task。
/// - 延迟 persist：文件创建推迟到显式 `persist()` 调用。
/// - 后台写入：Tokio task 持有文件句柄，调用方不阻塞 I/O。
pub struct JournalRecorder {
    tx: mpsc::Sender<JournalCmd>,
    writer_task: Arc<JournalWriterTask>,
    journal_path: PathBuf,
}

enum JournalCmd {
    AddEvents(Vec<RunEvent>),
    Persist { ack: oneshot::Sender<io::Result<()>> },
    Flush { ack: oneshot::Sender<io::Result<()>> },
    Shutdown { ack: oneshot::Sender<io::Result<()>> },
}

struct JournalWriterTask {
    handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    terminal_failure: Mutex<Option<Arc<io::Error>>>,
}

impl JournalRecorder {
    /// 创建新的 journal recorder。不立即创建文件。
    pub fn new(path: PathBuf) -> io::Result<Self> { /* ... */ }

    /// 从已有 journal 恢复（追加模式）。
    pub fn resume(path: PathBuf) -> io::Result<Self> { /* ... */ }

    /// 记录一个事件到缓冲区。非阻塞。
    pub fn record(&self, event: RunEvent) {
        let _ = self.tx.try_send(JournalCmd::AddEvents(vec![event]));
    }

    /// 批量记录事件。
    pub fn record_batch(&self, events: Vec<RunEvent>) {
        let _ = self.tx.try_send(JournalCmd::AddEvents(events));
    }

    /// 持久化所有缓冲事件到文件。
    pub async fn persist(&self) -> io::Result<()> { /* ... */ }

    /// 等待所有已提交的写入完成。
    pub async fn flush(&self) -> io::Result<()> { /* ... */ }

    /// 关闭 journal，等待最后的写入完成。
    pub async fn shutdown(&self) -> io::Result<()> { /* ... */ }

    /// 生成 run receipt（包含 journal SHA-256 + hash chain 验证状态）。
    pub fn build_receipt(&self) -> io::Result<RunReceipt> { /* ... */ }
}

impl Clone for JournalRecorder {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            writer_task: self.writer_task.clone(),
            journal_path: self.journal_path.clone(),
        }
    }
}
```

**verifier.rs**（Gemini 模式 + orz hash chain）：

```rust
// orz-assurance/src/journal/verifier.rs

use super::event::RunEvent;

/// 四维机械不变量检查结果。
/// 前三维来自 Gemini CLI `checkContextInvariants`，
/// 第四维（hash chain）是 orz 新增。
#[derive(Debug, Default)]
pub struct JournalInvariants {
    pub duplicate_event_ids: Vec<String>,
    pub orphan_events: Vec<u64>,           // sequence indices without turn association
    pub timestamp_violations: Vec<u64>,    // indices where ts[i] < ts[i-1]
    pub hash_chain_breaks: Vec<u64>,       // indices where prev_sha256 mismatch
}

impl JournalInvariants {
    /// 单次扫描检查所有四个不变量。O(n)。纯诊断——从不修改或抛出。
    pub fn check(events: &[RunEvent]) -> Self {
        let mut invariants = Self::default();
        let mut seen_ids = std::collections::HashSet::new();

        for (i, event) in events.iter().enumerate() {
            // 1. 重复 ID
            if !seen_ids.insert(&event.event_id) {
                invariants.duplicate_event_ids.push(event.event_id.clone());
            }
            // 2. 孤儿 turn（没有关联 turn 的事件跳过）
            // 3. 时间戳线性（如果 i > 0）
            if i > 0 {
                if event.timestamp < events[i - 1].timestamp {
                    invariants.timestamp_violations.push(i as u64);
                }
                // 4. Hash chain continuity
                if event.previous_event_sha256.as_deref()
                    != Some(&events[i - 1].event_sha256)
                {
                    invariants.hash_chain_breaks.push(i as u64);
                }
            }
        }
        invariants
    }

    pub fn is_clean(&self) -> bool {
        self.duplicate_event_ids.is_empty()
            && self.orphan_events.is_empty()
            && self.timestamp_violations.is_empty()
            && self.hash_chain_breaks.is_empty()
    }
}
```

### 4.6 session/ — 会话管理模块

```rust
// orz-assurance/src/session/mod.rs

pub mod trust;      // workspace trust verification
pub mod envelope;   // security envelope (HMAC)
pub mod lifecycle;  // active → archiving → archived
pub mod snapshot;   // ═══ OpenCode 模式 shadow Git ═══
```

**snapshot.rs**（OpenCode 模式）：

```rust
// orz-assurance/src/session/snapshot.rs

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct SnapshotStore {
    git_dir: PathBuf,       // ~/.orz/snapshot/<project>/<hash(worktree)>
    worktree: PathBuf,
    semaphore: Arc<Semaphore>,  // 每个 store 一个锁
}

pub struct PatchResult {
    pub snapshot_hash: String,
    pub changed_files: Vec<PathBuf>,
}

impl SnapshotStore {
    /// 初始化 snapshot store。
    ///
    /// 1. 创建 `~/.orz/snapshot/<project>/<hash(worktree)>`
    /// 2. `git init` (GIT_DIR + GIT_WORK_TREE)
    /// 3. 写 `objects/info/alternates` → 指向源 repo
    /// 4. 配置 longpaths/manyFiles/index v4
    pub fn init(project_id: &str, worktree: &Path) -> Result<Self, AssuranceError> {
        // ...
    }

    /// 创建快照。返回 tree hash（snapshot handle）。
    /// 自动排除 >2MB 文件和 gitignore 匹配的文件。
    pub fn track(&self) -> Result<String, AssuranceError> {
        let _guard = self.semaphore.acquire().await?;
        // 1. sync() — 复制 exclude 规则
        // 2. add() — 过滤后 stage
        //    - diff-files --name-only → changed tracked
        //    - ls-files --others → untracked
        //    - check-ignore → 排除 ignored
        //    - >2MB → block + exclude
        //    - add --all --sparse
        // 3. write-tree → 返回 tree hash
        // ...
    }

    /// 获取自某快照以来的变更文件列表。
    pub fn patch(&self, snapshot_hash: &str) -> Result<PatchResult, AssuranceError> {
        let _guard = self.semaphore.acquire().await?;
        // add() → diff --cached --name-only <hash>
        // ...
    }

    /// 将整个 worktree 恢复到指定快照。
    pub fn restore(&self, snapshot_hash: &str) -> Result<(), AssuranceError> {
        let _guard = self.semaphore.acquire().await?;
        // read-tree <hash> → checkout-index -a -f
        // ...
    }

    /// 选择性恢复：仅恢复指定文件。
    pub fn revert(&self, patches: &[PatchResult]) -> Result<(), AssuranceError> {
        let _guard = self.semaphore.acquire().await?;
        // 逐文件 checkout <hash> -- <file>
        // 批量：同 hash 邻接文件 batch ≤100
        // 冲突检测：父目录前缀 → 拆开
        // ...
    }
}

// 后台 GC：每小时 `git gc --prune=7.days`，启动后延迟 1 分钟。
```

### 4.7 credential/ — Windows Credential Manager

```rust
// orz-assurance/src/credential/mod.rs

use windows::Security::Credentials::*;

pub struct CredentialManager;

impl CredentialManager {
    /// 从 Windows Credential Manager 读取凭据。
    /// 目标格式: "FEP-Agent/DeepSeek"
    pub fn read(target: &str) -> Result<String, AssuranceError> {
        // PasswordVault::Retrieve -> credential.Password
        // 凭据不写入 journal（只存储 SHA-256）
        // ...
    }
}
```

### 4.8 sandbox/ — Windows Job Object

```rust
// orz-assurance/src/sandbox/mod.rs

pub mod job_object;  // windows_sandbox.py → Rust

// orz-assurance/src/sandbox/job_object.rs

pub struct JobObjectSupervisor {
    job_handle: windows::Win32::System::JobObjects::HANDLE,
}

impl JobObjectSupervisor {
    /// 创建 Job Object 并在创建时分配子进程
    /// (PROC_THREAD_ATTRIBUTE_JOB_LIST, GAK-WIN-001)
    pub fn new() -> Result<Self, AssuranceError> { /* ... */ }

    /// 在 Job Object 中运行命令
    pub fn contained_run(&self, cmd: &str, args: &[&str]) -> Result<ContainedProcess, AssuranceError> {
        // CREATE_SUSPENDED → AssignProcessToJobObject → ResumeThread
        // ...
    }
}
```

### 4.9 permit/ — One-shot action permit

```rust
// orz-assurance/src/permit/mod.rs

/// 一次性操作许可。每次危险 action 生成新的 request digest，
/// 不持久化长期 grant。
pub struct OneShotPermit {
    pub permit_id: String,
    pub action_digest: String,     // sha256(action_type + arguments + run_id)
    pub risk_class: RiskClass,
    pub granted_by: PermitAuthority,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub enum RiskClass {
    Low,       // read-only, no side effects
    Medium,    // file modifications within workspace
    High,      // network, credential, destructive
    Critical,  // irreversible external side effects
}

pub enum PermitAuthority {
    UserExplicit,   // 用户交互式确认
    PolicyRule,      // 预定义策略规则
    MechanicalBlock, // 机械阻断（不可被用户覆盖）
}
```

## 5. 双二进制入口

### 5.1 orz-bin — 自研 TUI 入口

```rust
// crates/orz-bin/src/main.rs
//
// 产出 `orz` 二进制。基于 xai-grok-pager-bin/src/main.rs 裁剪重构。
// 链接 orz-tui (assurance workbench)。

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 保留的 Grok 初始化路径 ──
    // - panic/allocator scaffolding
    // - CLI parse
    // - tokio runtime
    //
    // ── 移除 ──
    // - telemetry/sentry init
    // - auth/login/logout
    // - update channel
    // - managed config / remote settings
    //
    // ── 新增 ──
    // - orz_assurance 模块初始化
    // - gate registry 注册
    // - 启动 orz-tui (assurance workbench)

    let runtime = build_tokio_runtime()?;
    runtime.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let args = OrzArgs::parse_cli();

    match args.command {
        Command::Agent { mode, .. } => {
            // stdio / headless / leader
            // 保留 Grok 的 agent 启动逻辑
            // assurance 在 shell 层自动注入
            run_agent_command(args).await
        }
        // ... 其余命令
        _ => {
            // fallthrough: 启动 orz-tui
            orz_tui::app::run(args).await
        }
    }
}
```

### 5.2 grok 二进制 — Grok 原生 TUI 兜底入口

```rust
// crates/codegen/xai-grok-pager-bin/src/main.rs
//
// 产出 `grok` 二进制。保留 Grok 原生 TUI 作为兜底。
// 与 orz-bin 链接完全相同的 orz-shell/orz-tools/orz-assurance。
// 区别：启动 xai-grok-pager (Grok 原生 TUI) 而非 orz-tui。

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 与 orz-bin 相同的初始化 ──
    // - panic/allocator scaffolding
    // - tokio runtime
    //
    // ── 移除 ──
    // - telemetry/sentry init
    // - auth/login/logout
    // - update channel
    // - managed config / remote settings
    //
    // ── 注意 ──
    // - orz-assurance 同样初始化（同一个 assurance 层）
    // - 但启动 Grok 原生 TUI（不是 orz-tui）
    // - assurance gate/joural 在后台静默运行

    let runtime = build_tokio_runtime()?;
    runtime.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let args = GrokArgs::parse_cli();  // 保留 Grok 的参数名

    match args.command {
        Command::Agent { mode, .. } => {
            // 完全相同的 agent 启动逻辑（assurance 自动注入）
            run_agent_command(args).await
        }
        _ => {
            // fallthrough: 启动 Grok 原生 TUI
            // assurance 在 shell 层自动运行，但 TUI 不展示 assurance 面板
            xai_grok_pager::app::run(args).await
        }
    }
}
```

## 6. 构建系统变更

### 6.1 Workspace Cargo.toml

```toml
[workspace]
members = [
    # ── orz 自研 ──
    "crates/orz-bin",
    "crates/orz-assurance",
    "crates/orz-tui",

    # ── 重命名的 Grok crates ──
    "crates/orz-shell",       # was xai-grok-shell
    "crates/orz-tools",       # was xai-grok-tools
    "crates/orz-tools-api",
    "crates/orz-workspace",   # was xai-grok-workspace
    "crates/orz-workspace-client",
    "crates/orz-workspace-types",
    "crates/orz-sandbox",     # was xai-grok-sandbox
    "crates/orz-mcp",         # was xai-grok-mcp
    "crates/orz-config",      # was xai-grok-config
    "crates/orz-config-types",
    "crates/orz-http",        # was xai-grok-http
    "crates/orz-models",      # was xai-grok-models
    "crates/orz-hooks",       # was xai-grok-hooks
    "crates/orz-markdown",    # was xai-grok-markdown
    "crates/orz-markdown-core",
    "crates/orz-paths",
    "crates/orz-env",
    "crates/orz-shared",
    "crates/orz-version",
    "crates/orz-agent",
    "crates/orz-memory",      # was xai-grok-memory (保留, 默认关闭)

    # ── 保留原名的 Grok TUI crates（兜底 UI） ──
    "crates/codegen/xai-grok-pager",
    "crates/codegen/xai-grok-pager-render",
    "crates/codegen/xai-grok-pager-minimal",
    "crates/codegen/xai-grok-pager-bin",

    # ── 其余保留 crates ──
    "crates/codegen/xai-agent-lifecycle",
    # ... remaining kept crates
]
resolver = "2"

# 双二进制声明
[[bin]]
name = "orz"
path = "crates/orz-bin/src/main.rs"

[[bin]]
name = "grok"
path = "crates/codegen/xai-grok-pager-bin/src/main.rs"
```

### 6.2 CI 变更

```yaml
# .github/workflows/ci.yml
jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release
      - run: cargo test --all

  conformance:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release
      - name: Run Python conformance suite against orz binary
        run: |
          pip install -r requirements.txt
          python -m pytest assurance/tests/ --orz-binary ./target/release/orz.exe
```

## 7. 注入点标记规范

所有源码修改处使用统一注释标记，便于 upstream rebase 时定位：

```rust
// ── ORZ-INJECTION: <name> (injection point <N>) ──
// Replaces: <被替换的旧 workaround (如有)>
// <修改代码>
// ── END ORZ-INJECTION ──
```

已定义的注入点（`orz` 和 `grok` 两条路径均生效，因为注入在 shell/tools 层）：

| ID | 名称 | 文件 | orz 路径 | grok 路径 | 替换的旧 workaround |
|----|------|------|---------|----------|-------------------|
| IP1 | thinking-disabled | orz-models / sampler | ✅ | ✅ | `scripts/deepseek_thinking_proxy.py` |
| IP2a | tool-availability-context | orz-shell mvp_agent/mod.rs | ✅ | ✅ | Python sidecar 文本注入 |
| IP2b | orientation-checkpoint | orz-shell mvp_agent/mod.rs | ✅ | ✅ | `fixed_step_interval` 触发 |
| IP2c | info-sufficiency-check | orz-shell mvp_agent/mod.rs | ✅ | ✅ | 新功能 |
| IP3a | ipg-gate | orz-tools bridge.rs | ✅ | ✅ | 当前 gsa ask/chat 不跑 IPG |
| IP3b | orientation-counters | orz-tools bridge.rs | ✅ | ✅ | 新功能 |
| IP3c | sufficiency-trigger | orz-tools bridge.rs | ✅ | ✅ | 新功能 |
| IP4a | session-lifecycle | orz-shell acp_agent.rs | ✅ | ✅ | Python GrokAcpSession |
| IP4b | prompt-lifecycle | orz-shell acp_agent.rs | ✅ | ✅ | 无 |
| IP4c | session-close | orz-shell acp_agent.rs | ✅ | ✅ | Python sidecar receipt |
| IP5 | pre-mutation-snapshot | orz-workspace | ✅ | ✅ | 无 |
| IP6 | hard-gate-enforcement | orz-hooks + permission | ✅ | ✅ | hook fail-open 绕过 |

## 8. 实现顺序

### Phase 1: Scaffold（目标：`cargo build` 产出 `orz.exe` + `grok.exe`）

1. Clone Grok Build `500129c7`
2. 删除 §1.1 中的 18 crates
3. 重命名 §1.2 中的 crate 名（`xai-grok-*` → `orz-*`），保留 Grok TUI crates 原名
4. 创建 `orz-bin/`, `orz-assurance/` (空骨架), `orz-tui/` (空骨架)
5. 修改 `orz-bin/src/main.rs`：裁剪 pager-bin 的 main，移除 telemetry/auth/update，链接 orz-tui
6. 修改 `xai-grok-pager-bin/src/main.rs`：同样裁剪，但保留 Grok TUI 启动路径
7. 添加 memory 开关：`config.toml` 中 `memory.enabled = false`
8. 更新 workspace `Cargo.toml`：双二进制 + 全部 crate
9. **验证**: `cargo build --release` 成功, `./target/release/orz --version`, `./target/release/grok --version`

### Phase 2: Journal + Transport（目标：`orz -p "hello"` → valid `events.jsonl`）

1. 实现 `orz-assurance::journal::event` (RunEvent)
2. 实现 `orz-assurance::journal::chain` (SHA-256 hash chain)
3. 实现 `orz-assurance::journal::recorder` (async channel)
4. IP1: HTTP transport `thinking: disabled` 注入
5. IP4a/IP4c: session lifecycle journal events
6. **验证**: journal hash chain 连续, Python verifier 可独立验证

### Phase 3: Gates + Orientation（目标：完整 gate 链）

1. 实现 `orz-assurance::gates` (ipg, tool_availability, source_visibility, adapter)
2. 实现 `orz-assurance::orientation` (checkpoint, stagnation, sufficiency, trigger)
3. IP2a/IP2b/IP2c: prompt pipeline 注入
4. IP3a/IP3b/IP3c: tool dispatch 注入
5. IP6: hard gate enforcement
6. **验证**: `orz` 交互式会话通过完整 gate 链

### Phase 4: Snapshot + TUI + Polish

1. 实现 `orz-assurance::session::snapshot` (shadow Git)
2. IP5: pre-mutation snapshot
3. 从 Python `assurance/tui/` 移植 TUI → `orz-tui/`
4. 实现 `orz-assurance::session` (trust, envelope, lifecycle)
5. 实现 `orz-assurance::sandbox::job_object`
6. 实现 `orz-assurance::credential`
7. 实现 `orz-assurance::permit`
8. Python 项目标记 `reference-spec`
9. **验证**: orz 全功能通过 Python conformance suite
