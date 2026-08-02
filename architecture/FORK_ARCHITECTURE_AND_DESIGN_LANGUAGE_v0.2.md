# Fork 架构与统一设计语言 v0.2

状态：2026-08-02 初版。从 v0.1 的"以 Grok 源码为模板注入"升级为"拆解 Grok、混搭重构"——
Grok 是参考框架而非神圣上游。GSA 从 Grok 中取用成熟组件，替换和注入自己的 assurance 层，
编译为独立二进制 `gsa`。

## 1. 决策升级：从"Fork Grok"到"拆解 Grok"

v0.1 的定位是 fork Grok → 注入 assurance → 编译为 GSA。v0.2 的定位更激进：

```
v0.1 (fork):                      v0.2 (dismantle):
  Grok 源码                         Grok 源码（只读参考）
     │                                  │
     ├── 注入 assurance                 ├── 取用成熟组件（ACP/tool/workspace/sandbox）
     └── 编译为 gsa                     ├── 替换 UI 层（GSA TUI）
                                        ├── 注入 assurance（编译时，非外部）
                                        ├── 移除不需要的（telemetry/cloud/auth）
                                        └── 编译为 gsa（独立二进制，不是 Grok 变体）
```

**核心理由**：

1. ADR-0003 已裁决：Grok 是 reference runtime，不是强制底座。GSA 有权选择取用什么、替换什么。
2. 当前 Python assurance 层已有完整 gate、journal、orientation、TUI 原型——
   这些不是 Grok 的"扩展"，而是 GSA 的产品差异核心。
3. Grok 中的 telemetry、cloud auth、vendor update、feedback 等组件对 GSA 是 dead code。
4. "混搭"意味着：取 Grok 的 ACP + tool runtime + workspace，用自己的 assurance kernel + TUI +
   DeepSeek adapter，不强制保持 Grok 的 crate 边界或内部 API。

## 2. 组件分级：Keep / Modify / Replace / Add / Remove

### 2.1 KEEP — 直接采用，不修改

Grok 中这些组件成熟且与 assurance 无关，直接作为 GSA 的子 crate 使用：

| Grok crate | 功能 | 采用理由 |
|-----------|------|---------|
| `xai-grok-shell` | Agent runtime loop, ACP JSON-RPC stdio, headless entry point | 成熟的 model↔tool 循环，ACP 协议完整 |
| `xai-grok-tools` | terminal, file edit, search, web fetch, grep, glob | 工具实现不涉及 assurance 决策 |
| `xai-grok-workspace` | filesystem ops, VCS, execution, checkpoint | 工作区操作是通用能力 |
| ACP protocol stack | JSON-RPC 2.0, session/new, session/prompt, session/update | 协议层不包含安全策略 |
| sandbox infrastructure | OS sandbox, filesystem isolation | 物理隔离机制，与 policy 分离 |
| MCP protocol support | Model Context Protocol | 工具扩展协议，保持兼容 |
| config/TOML parsing | 配置解析 | 纯工程组件 |
| markdown rendering | 终端渲染 | 纯 UI 组件 |

**约束**：这些 crate 的内部 API 可能随上游变化。GSA 锁定当前 revision 的代码，
后续上游更新按 UPSTREAM_VERSION_STRATEGY 的三轨制（historical/current/selected）评估。

### 2.2 MODIFY — 保留骨架，注入 assurance

在这些组件中保持核心逻辑，但在关键路径注入 GSA assurance：

| 组件 | 修改位置 | 注入内容 |
|------|---------|---------|
| HTTP transport (`model/`) | Chat Completions 请求构造 | `thinking: disabled`（替换 thinking proxy daemon） |
| Prompt pipeline (`prompt/`) | system prompt 组装点 | `[TOOL_AVAILABILITY]` context block（会话开始一次） |
| Prompt pipeline | 动作间隙 | `[ORIENTATION_CHECKPOINT]` + `[CHECKLIST_CONTEXT]`（事件驱动触发） |
| Prompt pipeline | 检索完成后 | `[INFO_SUFFICIENCY_CHECK]` |
| Tool dispatch (`tools/`) | 工具执行前 | IPG 检查 + action authorization |
| Tool dispatch | 工具执行后 | orientation 计数器更新（token/action/tool variety） |
| Tool dispatch | 检索工具完成后 | INFO_SUFFICIENCY_CHECK 触发标记 |
| Session lifecycle | session 创建 | workspace trust 验证, security envelope 初始化, `run_preflight` |
| Session lifecycle | 每个事件 | append hash-chained JSONL line |
| Session lifecycle | session 关闭 | `run_finished`, journal seal, receipt 生成 |
| Permission system | permission 决策后 | hard gate 不可被 hook fail-open 绕过 |
| Composition root (`*-bin`) | 入口 | 初始化 assurance 模块, 注册 gates, 启动 journal |

**修改原则**：
- 修改点必须是最窄的——函数前置/后置 hook，不重写整个模块。
- 每个注入点有对应的 feature flag，可以在 `Cargo.toml` 中禁用（用于测试对照）。
- 上游代码的修改用 `// GSA-INJECTION:` 注释标记，便于后续 upstream rebase 时定位。

### 2.3 REPLACE — 完全替换

| Grok 原组件 | GSA 替换 | 理由 |
|------------|---------|------|
| `xai-grok-pager` (TUI) | `gsa-tui` (新 crate) | GSA TUI 是 assurance workbench，展示 gate 状态、journal 投影、orientation checkpoint；不是 Grok 的通用聊天 UI |
| `xai-grok-pager-bin` | `gsa-bin` (新 crate) | 新的 composition root，初始化 assurance 而非 Grok 的 telemetry/cloud |
| 配置文件 (`config.toml`) | GSA profile system | profile 按 capability/gate 选择，不是 Grok 的 model/provider config |

### 2.4 ADD — 全新实现（从 Python spec 移植）

```
gsa-assurance/                         # 编译产物的一部分，不是外部
├── Cargo.toml
├── src/
│   ├── lib.rs                         # assurance 模块注册
│   │
│   ├── gates/                         # 门禁（从 Python assurance/gates/ 移植）
│   │   ├── mod.rs
│   │   ├── ipg.rs                     # instruction_provenance_gate.py
│   │   ├── tool_availability.rs       # tool_availability_gate.py
│   │   ├── source_visibility.rs       # source_visibility.py
│   │   └── adapter.rs                 # adapter_gate.py
│   │
│   ├── orientation/                   # 中立问询（从 orientation_runtime_guard.py 移植 + 新设计）
│   │   ├── mod.rs
│   │   ├── checkpoint.rs             # build_orientation_checkpoint
│   │   ├── stagnation.rs             # evaluate_runtime_stagnation_guard
│   │   ├── sufficiency.rs            # INFO_SUFFICIENCY_CHECK（新）
│   │   └── trigger.rs                # OrientationTrigger 事件驱动计数器（新）
│   │
│   ├── journal/                       # 事件日志（从 journal.py, canonical_cli.py 移植）
│   │   ├── mod.rs
│   │   ├── event.rs                   # RunEvent 类型定义
│   │   ├── chain.rs                   # SHA-256 hash chain
│   │   └── verifier.rs               # 独立重建 + 交叉验证
│   │
│   ├── session/                       # 会话管理
│   │   ├── mod.rs
│   │   ├── trust.rs                   # workspace trust (workspace_trust.py)
│   │   ├── envelope.rs               # security envelope (envelope.py)
│   │   └── lifecycle.rs              # active → archiving → archived
│   │
│   ├── sandbox/                       # 沙箱（Windows）
│   │   ├── mod.rs
│   │   └── job_object.rs             # windows_sandbox.py → Rust
│   │
│   ├── credential/                    # 凭据
│   │   └── mod.rs                     # Windows Credential Manager
│   │
│   └── permit/                        # 一次性许可
│       └── mod.rs                     # one-shot action permit
```

### 2.5 REMOVE — 从源码树中删除

| Grok 组件 | 删除理由 |
|-----------|---------|
| telemetry / analytics | 本地审计优先，不外发 |
| cloud authentication | GSA 是 local-only，不需要 X.AI 登录 |
| vendor update channel | 版本管理由 GSA 三轨制控制 |
| feedback / rating | 不收集用户反馈 |
| remote relay / cloud service | ADR-0002 已延期 |
| external IDE integrations | 只保留 VS Code extension（已有） |
| `xai-grok-pager` TUI | 被 gsa-tui 替换 |
| `xai-grok-pager-bin` | 被 gsa-bin 替换 |

## 3. 最终 Crate 结构

```
gsa/                                  # Git repo: GSA（从 grok-build fork+拆解）
├── Cargo.toml                        # workspace root
├── SOURCE_REV                        # 记录原始 Grok monorepo commit
├── README.md
│
├── crates/
│   ├── gsa-bin/                      # ═══ 新: composition root ═══
│   │   ├── Cargo.toml
│   │   └── src/main.rs               # 入口：初始化 assurance → 启动 shell
│   │
│   ├── gsa-tui/                      # ═══ 新: GSA assurance TUI ═══
│   │   ├── Cargo.toml
│   │   └── src/                      # 从 Python assurance/tui/ 移植设计
│   │       ├── lib.rs
│   │       ├── app.rs                # TuiPrototype → Rust
│   │       ├── widgets/              # assurance 专用 widget
│   │       ├── event_source.rs       # FakeEventSource, LiveRunEventSource
│   │       └── bridge.rs             # Grok ACP → TUI event bridge
│   │
│   ├── gsa-assurance/                # ═══ 新: assurance kernel ═══
│   │   ├── Cargo.toml
│   │   └── src/                      # 见 §2.4 结构
│   │
│   ├── xai-grok-shell/               # ← KEEP (modify injection points)
│   ├── xai-grok-tools/               # ← KEEP
│   ├── xai-grok-workspace/           # ← KEEP
│   ├── xai-grok-sandbox/             # ← KEEP
│   ├── xai-grok-mcp/                 # ← KEEP
│   ├── xai-grok-config/              # ← KEEP
│   ├── xai-grok-markdown/            # ← KEEP
│   └── ... (other kept crates)
│
├── schemas/                          # JSON Schema（从 Python 项目迁移）
│   ├── run-manifest-v0.1.schema.json
│   ├── run-event-v0.1.schema.json
│   ├── orientation-checkpoint-v0.1.schema.json
│   ├── orientation-checkpoint-verification-v0.1.schema.json
│   ├── runtime-stagnation-guard-receipt-v0.1.schema.json
│   ├── gate-decision-v0.1.schema.json
│   ├── grok-runtime-receipt-v0.1.schema.json
│   └── ...
│
├── fixtures/                         # 测试 fixture（从 Python 迁移 + 新）
│   ├── positive/
│   └── negative/
│
└── tests/                            # 集成测试
    ├── gate_tests/
    ├── orientation_tests/
    ├── journal_tests/
    └── integration_tests/
```

## 4. 注入点详细设计

### 4.1 注入点 1: HTTP 传输层 → thinking disabled

**原始位置**：`xai-grok-shell` 中构造 Chat Completions 请求 body 的位置

**当前 workaround**：`scripts/deepseek_thinking_proxy.py`（外部 HTTP 代理，390 行）

**注入方案**：
```rust
// gsa-assurance 提供:
pub fn assure_request_body(mut body: serde_json::Value) -> serde_json::Value {
    body["thinking"] = json!({"type": "disabled"});
    body
}

// 在 shell 的 HTTP 请求构造点调用:
let body = gsa_assurance::assure_request_body(body);
```

**验证**：移除 thinking proxy daemon，直接用 `gsa -p "hello"` 验证 DeepSeek 不返回 thinking content。

### 4.2 注入点 2: Prompt 管线 → assurance context blocks

**注入位置**：system prompt / user prompt 组装点

**触发逻辑**由 `gsa-assurance::orientation::trigger` 提供：

```rust
// gsa-assurance::orientation::trigger
pub struct OrientationTrigger {
    pub turn_output_tokens: u64,
    pub turn_action_count: u64,
    pub distinct_tool_types: u64,
    pub conversation_turn: u64,
    pub cooldown_remaining: u64,
}

impl OrientationTrigger {
    pub fn should_fire(&self) -> bool {
        // 轮次触发 — 不受 cooldown
        if self.conversation_turn > 0 && self.conversation_turn % 8 == 0 {
            return true;
        }
        if self.cooldown_remaining > 0 {
            return false;
        }
        self.turn_output_tokens >= 7000
            || self.turn_action_count >= 10
            || self.distinct_tool_types >= 7
    }

    pub fn record_action(&mut self) {
        self.turn_action_count += 1;
        if self.cooldown_remaining > 0 {
            self.cooldown_remaining -= 1;
        }
    }

    pub fn record_tokens(&mut self, count: u64) {
        self.turn_output_tokens += count;
    }

    pub fn record_tool_type(&mut self, tool_id: &str) {
        // distinct tool types tracked separately
    }

    pub fn next_turn(&mut self) {
        self.conversation_turn += 1;
        self.turn_output_tokens = 0;
        self.turn_action_count = 0;
        self.distinct_tool_types = 0;
    }

    pub fn reset_cooldown(&mut self) {
        self.cooldown_remaining = 4; // 触发后冷却 4 个 action
    }
}
```

**在 shell 的动作间隙调用**：
```rust
// 工具执行完成后，模型调用前:
if trigger.should_fire() {
    let checkpoint = gsa_assurance::orientation::checkpoint::build(
        task_id, trigger.conversation_turn, ...);
    system_prompt.push_str(&checkpoint.message_block);
    trigger.reset_cooldown();
}
trigger.record_action();
```

**INFO_SUFFICIENCY_CHECK 独立触发**（检索完成后，不受 cooldown）：
```rust
// 检索工具完成后:
if tool_is_retrieval {
    let check = gsa_assurance::orientation::sufficiency::build(
        sources_count, categories, full_text_count);
    system_prompt.push_str(&check);
}
```

### 4.3 注入点 3: 工具调度 → IPG + action authorization

```rust
// 工具执行前:
let ipg_result = gsa_assurance::gates::ipg::evaluate(
    &instruction_context, &tool_call)?;
match ipg_result.decision {
    GateDecision::Block => return Err(AssuranceError::GateBlocked(ipg_result)),
    GateDecision::Warn => log::warn!("IPG warning: {:?}", ipg_result.reason_codes),
    _ => {}
}

// 工具执行后:
trigger.record_action();
trigger.record_tool_type(&tool_call.tool_id);
```

### 4.4 注入点 4: 会话生命周期 → journal

```rust
// session 创建:
let manifest = build_run_manifest(...);
journal::write_event(&mut journal, RunEvent {
    event_type: "run_preflight".into(),
    ...
})?;

// 每个事件:
journal::append(&mut journal, &event)?; // hash-chained, atomic

// session 关闭:
journal::write_event(&mut journal, RunEvent {
    event_type: "run_finished".into(),
    ...
})?;
journal::seal(&mut journal)?;
let receipt = journal::build_receipt(&journal)?;
```

## 5. 统一设计语言

### 5.1 命名约定

| 概念 | Rust 路径 | Python 来源 |
|------|----------|------------|
| Assurance kernel | `gsa_assurance::` | `assurance/` |
| Gate 类型 | `XxxGate` trait impl | 对应 `*_gate.py` |
| Gate 决策 | `GateDecision` enum | `PROTOCOL_DRAFT §9` |
| Event 类型 | `RunEvent` struct | `run-event-v0.1.schema.json` |
| Orientation checkpoint | `orientation::checkpoint` | `orientation_runtime_guard.py` |
| Stagnation guard | `orientation::stagnation` | `orientation_runtime_guard.py` |
| Info sufficiency | `orientation::sufficiency` | 新设计 |
| Trigger | `orientation::trigger` | 新设计（事件驱动） |
| Journal | `journal::` | `journal.py`, `canonical_cli.py` |
| Hash chain | `journal::chain` | `utils.py` — `sha256_bytes` |
| Security envelope | `session::envelope` | `envelope.py` |
| Workspace trust | `session::trust` | `workspace_trust.py` |
| Job object | `sandbox::job_object` | `windows_sandbox.py` |
| Credential | `credential::` | `deepseek_adapter.py` |
| One-shot permit | `permit::` | P3 permit 逻辑 |

### 5.2 Gate trait

```rust
/// 所有 gate 实现此 trait。
pub trait AssuranceGate {
    type Context;
    type Receipt: Serialize + Deserialize;

    /// 根据上下文评估 gate。决策不依赖模型行为。
    fn evaluate(&self, ctx: &Self::Context) -> Result<Self::Receipt, AssuranceError>;

    /// 独立验证 receipt——可在不同进程中重建。
    fn verify(receipt: &Self::Receipt, ctx: &Self::Context) -> Result<bool, AssuranceError>;
}
```

### 5.3 GateDecision 枚举

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateDecision {
    /// 证据充分，无顾虑
    Pass,
    /// 通过但有保留（例如覆盖不完整但不阻塞）
    Warn { reason_codes: Vec<String> },
    /// 硬停止，不得继续
    Block { reason_codes: Vec<String> },
    /// 证据不足，需更多信息
    Defer { missing: Vec<String> },
    /// Gate 不适用于当前 action
    NotApplicable,
}
```

### 5.4 RunEvent 结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub schema_version: String,          // "0.1.0"
    pub run_id: String,
    pub event_id: String,
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub run_manifest_sha256: String,
    pub previous_event_sha256: Option<String>,
    pub payload_schema: String,
    pub payload: serde_json::Value,
    pub payload_sha256: String,
    pub redaction: RedactionLevel,
    pub event_sha256: String,
}
```

### 5.5 不可变约束（编译时/启动时机械检查）

1. 每个 action 恰好一个 terminal event
2. Journal hash chain 连续不可断裂
3. `run_finished` 必须是最后一个 event
4. 凭据不写入 journal（只允许 SHA-256）
5. Hidden reasoning 不进入公开输出
6. Orientation checkpoint 不生成 counterexample_candidate
7. 用户授权可满足 permission，不能证明 provenance/coverage/independence

## 6. 迁移路线（4 Phase）

### Phase 1: Scaffold + Build（目标：gsa 可编译）

1. Fork Grok Build 源码（基于 lock 中的 `500129c7`）
2. 建立 `crates/gsa-bin/`（composition root，先只是透传 shell）
3. 建立 `crates/gsa-assurance/`（空模块骨架）
4. 建立 `crates/gsa-tui/`（空模块骨架）
5. 更新 workspace `Cargo.toml`
6. 确认 `cargo build` 产出 `gsa.exe`
7. 移除 telemetry/cloud/auth/dead crates
8. **验证**: `gsa --version` 输出 GSA 版本而非 Grok 版本

### Phase 2: Inject + Journal（目标：journal 可用）

1. 注入点 1: HTTP 传输层 `thinking: disabled`（删除 thinking proxy daemon）
2. 注入点 4: 会话生命周期 journal（`run_preflight` → `run_finished`）
3. 实现 `gsa-assurance::journal`（event, chain, verifier）
4. 迁移 schemas 到 Rust 项目
5. **验证**: `gsa -p "hello"` 产生有效的 `events.jsonl`，hash chain 可独立验证

### Phase 3: Gates + Orientation（目标：完整 gate 链）

1. 注入点 2: Prompt 管线 — tool_availability context + orientation checkpoint
2. 注入点 3: 工具调度 — IPG + action authorization
3. 实现 `gsa-assurance::gates`（ipg, tool_availability, source_visibility, adapter）
4. 实现 `gsa-assurance::orientation`（checkpoint, stagnation, sufficiency, trigger）
5. 实现事件驱动触发（替换 fixed_step_interval）
6. 实现硬门控计数器
7. 实现 INFO_SUFFICIENCY_CHECK
8. **验证**: `gsa` 交互式会话通过完整 gate 链

### Phase 4: TUI + Polish（目标：完整产品）

1. 从 Python `assurance/tui/` 移植 GSA TUI 到 `gsa-tui`
2. TUI 展示 gate 状态、journal 投影、orientation checkpoint
3. 实现 `gsa-assurance::session`（trust, envelope, lifecycle）
4. 实现 `gsa-assurance::sandbox::job_object`
5. 实现 `gsa-assurance::credential`
6. 实现 `gsa-assurance::permit`
7. Python 项目标记为 `reference-spec`
8. 更新 CI 为 Rust + Python conformance
9. **验证**: `gsa` 全功能通过 Python conformance suite

## 7. Python 项目的新角色

Fork 完成后，`D:\CLI`（Python）的角色：

| 角色 | 说明 |
|------|------|
| **Assurance spec reference** | Python 实现是 Rust 实现的"金版"——行为正确性由 Python 测试定义 |
| **Conformance test suite** | Rust `gsa` 必须通过 Python spec 的全部测试 |
| **Design documents** | 架构、协议、偏差记录、ADR |
| **Offline verification** | `canonical_cli.py` 保留为离线验证路径 |
| **Rapid prototyping** | 新 gate 行为先在 Python 中实现和验证，再移植 Rust |
| **Schema authority** | JSON Schema 的规范定义保持在 Python 项目中，Rust 项目复制 |

Python 实现**不删除**——它从"生产 runtime"降级为"reference spec + conformance suite"。

## 8. 与 v0.1 的关键差异

| | v0.1 | v0.2 |
|---|---|---|
| 对 Grok 的定位 | 源码模板，在其上注入 | 参考框架，拆解后取用 |
| TUI | 保留 Grok TUI + 可选 GSA 面板 | 完全替换为 GSA TUI |
| Composition root | 保留 Grok bin + 注入 | 替换为 gsa-bin |
| 移除组件 | 未明确 | 明确：telemetry/cloud/auth/feedback |
| Thinking proxy | fork 后移除 | Phase 2 明确删除 |
| Python 项目 | reference spec + conformance | 同左 + rapid prototype + schema authority |
| 混搭程度 | fork Grok，保持结构 | 拆解 Grok，重组为自己的结构 |

## 9. 风险与缓解

| 风险 | 缓解 |
|------|------|
| Grok 上游更新导致 merge conflict | 锁定 revision；三轨制评估；注入点用 `// GSA-INJECTION:` 标记 |
| Rust 移植工作量过大 | Phase 分步；Python 先作为 conformance spec；优先移植 gate/journal |
| Grok 内部 API 不稳定 | 不依赖未文档化的内部 API；必要时复制代码而非耦合 |
| Windows 构建复杂 | Phase 1 首先验证 Windows 编译；利用 Grok 已有的 build 脚本 |
| Python TUI → Rust TUI 重写 | TUI 设计（widget 结构、event model）已在 Python 中验证，Rust 是翻译 |

## 10. 参考

- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.1.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.1.md) — 上一版 fork 设计
- [`IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md`](IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md) — 偏差分析
- [`GROK_BUILD_ADAPTATION_v0.1.md`](GROK_BUILD_ADAPTATION_v0.1.md) — Grok crate 结构
- [`UPSTREAM_FIRST_INTEGRATION_v0.1.md`](UPSTREAM_FIRST_INTEGRATION_v0.1.md) — 职责边界
- [`PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md`](PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md) — 产品定位
- [`ADRs`](../adr/) — ADR-0001 (evidence-constrained kernel), ADR-0003 (runtime-neutral)
- [`OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md`](OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md) — 多源借鉴矩阵
- [`../upstream/grok-build.lock.json`](../upstream/grok-build.lock.json) — 当前锁定的 Grok 版本
