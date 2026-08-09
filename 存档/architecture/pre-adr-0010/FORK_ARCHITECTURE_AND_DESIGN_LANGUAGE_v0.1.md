# Fork 架构与统一设计语言 v0.1

> Archive metadata: original_path=`architecture/FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.1.md`; archived_at=`2026-08-09`; final_status=`superseded`; superseded_by=`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`; authority=`historical design input only`.


状态：2026-08-02。确定采用 fork 模式——以 Grok 源码为模板，注入 GSA assurance 层，
编译为 GSA 二进制。本文定义注入点、模块结构和统一设计语言。

## 1. 决策

```
当前 (sidecar):                   目标 (fork):
  GSA (Python) ─ACP─→ Grok          GSA (Rust, forked Grok)
  外部 gates                        内部 gates
  Grok 不知 GSA 存在                 assurance 是编译时注入
  grok 原生绕过 GSA                  gsa 命令就是 GSA
  两套 UI 不同 assurance            两套 UI 同一 assurance
```

不再维护 sidecar 模式的 thinking proxy daemon 等外部 workaround——
所有 assurance 能力在源码级解决。

## 2. 源模块结构

```
gsa/                                  # 编译产物: gsa (曾用名 grok)
├── Cargo.toml
├── src/
│   ├── main.rs                       # 入口 (保留原 Grok 启动路径)
│   │
│   ├── assurance/                    # ═══ GSA assurance 层 (新增) ═══
│   │   ├── mod.rs
│   │   │
│   │   ├── gates/                    # 门禁
│   │   │   ├── mod.rs
│   │   │   ├── ipg.rs                # 指令来源 / 反注入
│   │   │   ├── tool_availability.rs  # 工具可用性机械探测
│   │   │   ├── source_visibility.rs  # 来源全文可见性
│   │   │   └── adapter.rs            # 适配器调用 enforce
│   │   │
│   │   ├── orientation/              # 中立问询
│   │   │   ├── mod.rs
│   │   │   ├── checkpoint.rs         # 中途定位检查
│   │   │   ├── stagnation.rs         # 输出停滞检测
│   │   │   └── sufficiency.rs        # 信息收集确认
│   │   │
│   │   ├── journal/                  # 事件日志
│   │   │   ├── mod.rs
│   │   │   ├── event.rs              # run event 类型定义
│   │   │   ├── chain.rs              # hash chain (SHA-256, append-only)
│   │   │   └── verifier.rs           # 独立重建 + 交叉验证
│   │   │
│   │   ├── session/                  # 会话管理
│   │   │   ├── mod.rs
│   │   │   ├── trust.rs              # workspace trust
│   │   │   ├── envelope.rs           # security envelope (HMAC)
│   │   │   └── lifecycle.rs          # active → archiving → archived
│   │   │
│   │   ├── sandbox/                  # 沙箱
│   │   │   ├── mod.rs
│   │   │   └── job_object.rs         # Windows Job Object containment
│   │   │
│   │   ├── credential/               # 凭据
│   │   │   └── mod.rs                # Windows Credential Manager
│   │   │
│   │   └── permit/                   # 一次性许可
│   │       └── mod.rs                # one-shot action permit
│   │
│   ├── tui/                          # Grok TUI (保留，可选增加 GSA 面板)
│   ├── acp/                          # ACP 协议 (保留，GSA 的 agent 通信层)
│   ├── model/                        # ═══ 注入点 1: HTTP 传输 ═══
│   │   └── ...                       #    注入 thinking: disabled
│   ├── prompt/                       # ═══ 注入点 2: prompt 管线 ═══
│   │   └── ...                       #    注入 tool_availability context
│   │                                 #    注入 orientation checkpoint
│   ├── tools/                        # ═══ 注入点 3: 工具调度 ═══
│   │   └── ...                       #    注入 IPG + action authorization
│   └── session/                      # ═══ 注入点 4: 会话生命周期 ═══
│       └── ...                       #    注入 journal event 写入
│
├── schemas/                          # JSON Schema (从 Python 项目迁移)
│   ├── run-manifest-v0.1.schema.json
│   ├── run-event-v0.1.schema.json
│   ├── orientation-checkpoint-v0.1.schema.json
│   └── ...
│
├── fixtures/                         # 测试 fixture
│   ├── positive/
│   └── negative/
│
└── tests/                            # 集成测试
    └── ...
```

## 3. 注入点

### 注入点 1: HTTP 传输层

**位置**: `model/` 中构造 Chat Completions 请求 body 的位置

**注入内容**:
```rust
// 在序列化请求 body 前
body["thinking"] = json!({"type": "disabled"});
```

**替换**: 不再需要 thinking proxy daemon。

### 注入点 2: Prompt 管线

**位置**: 组装 system prompt / user prompt 的位置

**注入内容**:
- Session 开始时: `[TOOL_AVAILABILITY v0.1]` context block（仅一次）
- 动作间隙（硬门控触发）: `[ORIENTATION_CHECKPOINT v0.1]`
- 检索完成后: `[INFO_SUFFICIENCY_CHECK v0.1]`

**触发逻辑**: `assurance/orientation/` 中的计数器决定何时注入

### 注入点 3: 工具调度

**位置**: 工具执行前后

**注入内容**:
- 执行前: IPG 检查 + action authorization（P3 permit）
- 执行后: orientation 计数器更新（token 计数、action 计数、tool variety 统计）
- 检索工具完成后: 信息收集确认触发标记

### 注入点 4: 会话生命周期

**位置**: session 创建/关闭

**注入内容**:
- 创建: workspace trust 验证、security envelope 初始化、`run_preflight` event
- 每个事件: append hash-chained JSONL line
- 关闭: `run_finished` event、journal seal、receipt 生成

## 4. 统一设计语言

### 4.1 命名约定

| 概念 | 前缀/命名 | 示例 |
|------|----------|------|
| Assurance 模块 | `assurance::` | `assurance::gates::ipg` |
| Gate 类型 | `XxxGate` | `InstructionProvenanceGate` |
| Gate 结果 | `GateDecision` | `Allow / Warn / Defer / Block` |
| Event 类型 | snake_case, schema 同步 | `run_preflight`, `orientation_checkpoint` |
| Reason code | `REASON-COMPONENT-NNN` | `TOOL-BELIEF-AVAILABILITY-MISMATCH` |
| Receipt | `XxxReceipt` | `GateReceipt`, `RunReceipt` |
| 错误 | `AssuranceError` | 统一错误类型 |

### 4.2 Gate 模式

每个 gate 实现同一 trait：

```rust
trait AssuranceGate {
    type Context;
    type Receipt;

    fn evaluate(&self, ctx: &Self::Context) -> Result<Self::Receipt, AssuranceError>;
    fn verify(receipt: &Self::Receipt, ctx: &Self::Context) -> Result<bool, AssuranceError>;
}
```

### 4.3 事件模式

```rust
struct RunEvent {
    schema_version: String,       // "0.1.0"
    run_id: String,
    event_id: String,
    sequence: u64,
    timestamp: DateTime<Utc>,
    event_type: EventType,
    run_manifest_sha256: String,
    previous_event_sha256: Option<String>,
    payload: serde_json::Value,
    payload_sha256: String,
    redaction: RedactionLevel,    // metadata_only | summary | full
    event_sha256: String,         // computed from all above except self
}
```

### 4.4 硬门控模式

```rust
struct OrientationTrigger {
    turn_output_tokens: u64,      // 当前轮累计公开输出 token
    turn_action_count: u64,       // 当前轮累计动作数
    distinct_tool_types: u64,     // 当前轮不同 tool_id 去重计数
    conversation_turn: u64,       // 全局对话轮次
    cooldown_remaining: u64,      // 剩余冷却动作数
}

impl OrientationTrigger {
    fn should_fire(&self) -> bool {
        // 轮次触发 — 不受 cooldown 限制
        if self.conversation_turn > 0 && self.conversation_turn % 8 == 0 {
            return true;
        }
        // 动作间隙触发 — 受 cooldown 限制
        if self.cooldown_remaining > 0 {
            return false;
        }
        self.turn_output_tokens >= 7000
            || self.turn_action_count >= 10
            || self.distinct_tool_types >= 7
    }
}
```

### 4.5 不可变约束

以下约束在编译时或启动时由机械检查保证，不依赖模型行为：

1. 每个 action 恰好一个 terminal event
2. Journal hash chain 连续不可断裂
3. `run_finished` 必须是最后一个 event
4. 凭据不写入 journal（只允许 SHA-256）
5. Hidden reasoning 不进入公开输出
6. Orientation checkpoint 不生成 counterexample_candidate
7. 用户授权可以满足 permission 要求，但不能证明 provenance/coverage/independence

### 4.6 从 Python 到 Rust 的迁移映射

当前 Python 实现保留了完整的 assurance 逻辑和测试，作为 Rust 实现的**参考 spec**。
以下为关键映射：

| Python | Rust |
|--------|------|
| `instruction_provenance_gate.py` | `assurance::gates::ipg` |
| `tool_availability_gate.py` | `assurance::gates::tool_availability` |
| `source_visibility.py` | `assurance::gates::source_visibility` |
| `adapter_gate.py` | `assurance::gates::adapter` |
| `orientation_runtime_guard.py` | `assurance::orientation::checkpoint` |
| `canonical_cli.py` (journal 部分) | `assurance::journal` |
| `keystore.py` | `assurance::session::envelope` |
| `envelope.py` | `assurance::session::envelope` |
| `guarded_execution.py` | `assurance::sandbox` |
| `windows_sandbox.py` | `assurance::sandbox::job_object` |

Python 实现**不删除**——它继续作为：
1. Conformance fixture（验证 Rust 实现与 Python spec 一致）
2. 离线回归测试
3. 新 gate 行为的快速原型

## 5. 迁移路线

### Phase 1: Fork + Build
- Fork Grok 源码（`xai-org/grok-build`，commit `500129c7`）
- 重命名项目为 `gsa`
- 建立 `assurance/` 模块结构
- 确认可以编译为 `gsa.exe`

### Phase 2: 注入点 1 + 4（最低可用）
- HTTP 传输层注入 `thinking: disabled`
- 会话生命周期注入 journal（`run_preflight` → `run_finished`）
- 验证: `gsa -p "hello"` 产生有效的 events.jsonl

### Phase 3: 注入点 2 + 3（gate 接入）
- Prompt 管线注入 tool_availability context
- 工具调度注入 IPG + action authorization
- Orientation checkpoint（事件驱动触发）
- 验证: `gsa` 交互式会话通过完整 gate 链

### Phase 4: 对齐 + 清理
- TUI 添加 GSA assurance 面板
- 迁移所有 schema 到 Rust 项目
- Python 项目标记为 `reference-spec`
- 移除 thinking proxy daemon

## 6. 当前 Python 项目的角色

Fork 完成后，本仓库的角色变为：

- **Assurance spec reference** — Python 实现是 Rust 实现的"金版"
- **Conformance test suite** — Rust 实现必须通过 Python spec 的全部测试
- **Design documents** — 架构、协议、偏差记录
- **Offline verification** — Python 的 `canonical_cli.py` 保留为离线验证路径
