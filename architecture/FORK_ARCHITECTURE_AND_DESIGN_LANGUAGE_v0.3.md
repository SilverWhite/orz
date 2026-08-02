# Fork 架构与统一设计语言 v0.3

状态：2026-08-02。从 v0.2 的"拆解 Grok + 混搭"进一步融合 Codex CLI、
Gemini CLI、OpenCode、Goose 的成熟工程模式。同时项目从 GSA 更名为 **orz**（纯颜表情，无实际含义）。

## 0. 命名变更：GSA → orz

```
旧: GSA (General Scientific Assurance)
新: orz (无含义颜表情，打起来最顺手)

二进制:   gsa  → orz
crate:    gsa-* → orz-*
目录:     gsa/  → orz/
命令:     gsa   → orz
```

变更理由：GSA 全称过于自信（General Scientific Assurance），orz 无含义、简短、打字顺。

## 1. 设计融合：五源借鉴矩阵

本设计从五个成熟 CLI 产品中取用经过验证的工程模式。每个模式都有明确的来源、
在我们的 fork 中的落点，以及不采用的边界。

### 1.1 融合总览

```
                     orz (本产品)
                         │
    ┌────────────────────┼────────────────────┬────────────┬──────────────┐
    │                    │                    │            │              │
  Grok Build         Codex CLI          Gemini CLI     OpenCode       Goose
 (agent runtime)   (journal pattern)  (invariant check) (snapshot)  (security分离)
```

### 1.2 Codex CLI → journal 异步写入模式

**来源**：`codex-rs/rollout/src/recorder.rs` (Apache-2.0, commit `b9800de4`)

**取用模式**：

1. **RolloutRecorder** — 廉价可 clone 的 handle，内部用 bounded mpsc channel (256 slots)
   向后台 Tokio task 发送写入命令。调用方不阻塞在 I/O 上。

2. **延迟 persist** — 文件创建推迟到显式 `persist()` 调用。在此之前事件在 `pending_items` 中缓冲。
   允许 GSA 在 gate 通过前不落盘。

3. **单调序号** — `RolloutOrdinalState` 从已有 rollout 重建序号状态。Resume 时打开已有文件
   追加，不重写历史。

4. **SessionMeta 冻结** — 会话创建时一次性记录 agent nickname/role、model provider、
   capabilities、source、history mode/base。运行中不变。

**在我们的 fork 中的落点**：

```rust
// orz-assurance::journal::recorder

pub struct JournalRecorder {
    tx: Sender<JournalCmd>,
    writer_task: Arc<JournalWriterTask>,
    journal_path: PathBuf,
}

enum JournalCmd {
    AddEvents(Vec<RunEvent>),
    Persist { ack: oneshot::Sender<io::Result<()>> },
    Flush { ack: oneshot::Sender<io::Result<()>> },
    Shutdown { ack: oneshot::Sender<io::Result<()>> },
}

impl JournalRecorder {
    /// 廉价 clone——所有 clone 共享同一 channel 和 writer task。
    pub fn record(&self, event: RunEvent) { ... }
    pub fn persist(&self) -> io::Result<()> { ... }
    pub fn flush(&self) -> io::Result<()> { ... }
}
```

**不采用**：Codex 的多 agent 版本协商、SQLite state-db（我们只需要 JSONL）、
cloud session sync。

### 1.3 Gemini CLI → Context Graph Invariant Checker

**来源**：`packages/core/src/context/utils/invariantChecker.ts` (Apache-2.0, commit `acae7124`)

**取用模式**：

`checkContextInvariants` 做三个纯机械检查（单次扫描，O(n)，debug-only）：

1. **重复 ID** — `seenIds` set 检测，发现重复即 warn
2. **孤儿 turn** — 节点无 `turnId` 即孤儿
3. **时间戳线性** — 前向扫描，检测到 `timestamp[i] < timestamp[i-1]` 即 break

**在我们的 fork 中的落点**：

```rust
// orz-assurance::journal::verifier

pub struct JournalInvariants {
    pub duplicate_event_ids: Vec<String>,
    pub orphan_events: Vec<u64>,           // events without turn_id
    pub timestamp_violations: Vec<u64>,    // sequence indices where ts non-linear
    pub hash_chain_breaks: Vec<u64>,       // previous_event_sha256 mismatch
}

impl JournalInvariants {
    pub fn check(events: &[RunEvent]) -> Self {
        // single-pass scan of all events
        // purely diagnostic — never mutates, never throws
    }

    pub fn is_clean(&self) -> bool {
        self.duplicate_event_ids.is_empty()
            && self.orphan_events.is_empty()
            && self.timestamp_violations.is_empty()
            && self.hash_chain_breaks.is_empty()
    }
}
```

我们比 Gemini 多一个检查维度：**hash chain continuity**（`previous_event_sha256` 必须匹配前一个
event 的 `event_sha256`）。这四维检查组成 journal 的机械不变量校验。

**不采用**：Gemini 的 Episodic Context Graph 完整结构（我们只需要线性 event 序列）、
Google OAuth 认证。

### 1.4 OpenCode → 独立 Shadow Git Snapshot

**来源**：`packages/opencode/src/snapshot/index.ts` (MIT, commit `cb562b2c`)

**取用模式**：

1. **独立 Git store** — 快照存储在 `~/.orz/snapshot/<project-id>/<worktree-hash>`，
   绝不接触用户 `.git`。

2. **objects/info/alternates** — 指向源 repo 的 object database，避免重新 hash 已有 blob。
   对大型 repo 启动速度至关重要。

3. **文件大小上限** — 超过 2MB 的文件自动排除，加入 snapshot exclude 列表。

4. **信号量锁** — 每个 store 一个 `Semaphore(1)`，序列化所有 Git 操作，防止并发损坏。

5. **操作管线**：
   ```
   track()  →  add(过滤) → write-tree → 返回 tree hash (snapshot handle)
   patch(hash) → add → diff --cached <hash> → 返回变更文件列表
   restore(snapshot) → read-tree → checkout-index -a -f
   revert(patches) → 逐文件 checkout <hash> -- <file>
   ```

6. **后台 GC** — `git gc --prune=7.days`，每小时一次，启动后延迟 1 分钟。

7. **Git 配置** — 初始化时设置 `core.autocrlf=false`, `core.longpaths=true`,
   `feature.manyFiles=true`, `index.version=4` 等大型 worktree 优化。

**在我们的 fork 中的落点**：

```rust
// orz-assurance::session::snapshot

pub struct SnapshotStore {
    git_dir: PathBuf,         // ~/.orz/snapshot/<project>/<worktree>
    worktree: PathBuf,        // 实际工作目录
    semaphore: Semaphore,     // 单操作锁
}

impl SnapshotStore {
    pub fn init(project_id: &str, worktree: &Path) -> Result<Self> {
        // 1. 创建 ~/.orz/snapshot/<project>/<hash(worktree)>
        // 2. git init (with GIT_DIR/GIT_WORK_TREE env)
        // 3. 写 objects/info/alternates → 指向源 repo
        // 4. 配置 core.autocrlf=false, longpaths=true, manyFiles=true
    }

    pub fn track(&self) -> Result<String> {
        // add() → write-tree → 返回 tree sha
    }

    pub fn patch(&self, snapshot_hash: &str) -> Result<PatchResult> {
        // add() → diff --cached <hash> → 变更文件列表
    }

    pub fn restore(&self, snapshot_hash: &str) -> Result<()> {
        // read-tree <hash> → checkout-index -a -f
    }

    pub fn revert(&self, patches: &[PatchResult]) -> Result<()> {
        // 逐文件 checkout <hash> -- <file>
        // 批量优化：同 hash 的相邻文件 batch (上限 100)
        // 路径前缀冲突检测：父目录会影响子文件时拆开
    }
}
```

**与 assurance journal 的关系**——关键设计决策：

> Snapshot store 提供恢复能力；但**恢复是新的危险 action**。
> 每次 restore/revert 必须在 journal 中写入 `snapshot_restore` event，
> 包含 snapshot hash、restore scope、before/after digest、用户确认方式。
> 恢复成功不改变原 permission 决策，也不删除后续审计事件。

**不采用**：OpenCode 的云端基础设施（SST）、Bun 运行时、Nix flake 构建、
`structuredPatch` 大 context diff（我们用 journal digest 即可）。

### 1.5 Goose → Security Finding 与 Permission Decision 分离

**来源**：`crates/goose/src/security/security_inspector.rs` (Apache-2.0, commit `65e1e3d5`)

**取用模式**（Goose 的设计原则，非直接代码移植）：

> Scanner 输出 finding ID、evidence、confidence。
> Permission 另记 allow/deny/ask 和依据。
> 启发式 finding 不能伪装成安全证明，permission 也不能抹掉 finding。

**在我们的 fork 中的落点**（已在 assurance 设计中，这里明确为 invariants）：

```rust
// orz-assurance::gates

// LeakScanner / security inspector 产生:
pub struct SecurityFinding {
    pub finding_id: String,
    pub category: FindingCategory,    // secret_leak, prompt_injection, etc.
    pub evidence_sha256: String,
    pub confidence: Confidence,       // mechanical_detected | heuristic | model_assessed
    pub recommendation: String,       // suggested action, not binding
}

// Permission 决策独立记录:
pub struct PermissionDecision {
    pub request_id: String,
    pub tool_id: String,
    pub decision: GateDecision,
    pub basis: DecisionBasis,         // policy_rule | user_approved | mechanical_block
    pub related_findings: Vec<String>, // finding IDs, for audit only
}
// INVARIANT: SecurityFinding.confidence == heuristic 时
//   PermissionDecision.decision != Block (除非用户显式 block)
// INVARIANT: SecurityFinding 不自动产生 PermissionDecision
```

**不采用**：Goose 的可持久化 permission store（长期 grant 削弱逐 attempt 授权）、
specific permission expiration model。

### 1.6 融合边界：什么故意不取

```
Codex   ─ 不取: multi-agent version negotiation, SQLite state-db, cloud sync
Gemini  ─ 不取: Episodic Context Graph, OAuth, Vertex AI, GCP
OpenCode ─ 不取: SST cloud infra, Bun runtime, structuredPatch large context
Goose   ─ 不取: persistent permission store, expiration model, context_mgmt compaction
Aider   ─ 不取: repo map (与 FEP claim index 不同对象), architect/editor 双模型
Grok    ─ 不取: telemetry, cloud auth, vendor update, feedback, TUI
```

## 2. 组件分级（更新）

从 v0.2 的五级分类更新，新增 snapshot 子系统：

| 分级 | 组件 | 来源 |
|------|------|------|
| **KEEP** | ACP shell, tools, workspace, sandbox, MCP, config, markdown | Grok |
| **MODIFY** | HTTP transport, prompt pipeline, tool dispatch, session lifecycle, permission, composition root | Grok + orz injection |
| **REPLACE** | TUI → `orz-tui`, bin → `orz-bin`, config → profiles | orz |
| **ADD** | `orz-assurance` (7 子模块) | Python spec → Rust |
| **ADD** | `orz-assurance::journal::recorder` (async channel) | Codex 模式 |
| **ADD** | `orz-assurance::journal::verifier::JournalInvariants` | Gemini 模式 + hash chain |
| **ADD** | `orz-assurance::session::snapshot` (shadow Git) | OpenCode 模式 |
| **ADD** | SecurityFinding / PermissionDecision 分离 | Goose 原则 |
| **REMOVE** | telemetry, cloud auth, vendor update, feedback, remote relay | Grok dead code |

## 3. Crate 结构（更新）

```
orz/                                  # Git repo: orz
├── Cargo.toml                        # workspace root
├── SOURCE_REV                        # 记录原始 Grok monorepo commit
├── README.md
│
├── crates/
│   ├── orz-bin/                      # ═══ 新: composition root ═══
│   │   ├── Cargo.toml
│   │   └── src/main.rs               # 初始化 assurance → 注册 gates → 启动 shell
│   │
│   ├── orz-tui/                      # ═══ 新: assurance TUI ═══
│   │   └── src/                      # 从 Python assurance/tui/ 移植
│   │
│   ├── orz-assurance/                # ═══ 新: assurance kernel ═══
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── gates/
│   │       │   ├── mod.rs
│   │       │   ├── ipg.rs            # instruction_provenance_gate.py
│   │       │   ├── tool_availability.rs
│   │       │   ├── source_visibility.rs
│   │       │   └── adapter.rs
│   │       ├── orientation/
│   │       │   ├── mod.rs
│   │       │   ├── checkpoint.rs     # build_orientation_checkpoint
│   │       │   ├── stagnation.rs     # evaluate_runtime_stagnation_guard
│   │       │   ├── sufficiency.rs    # INFO_SUFFICIENCY_CHECK (新)
│   │       │   └── trigger.rs        # OrientationTrigger (事件驱动)
│   │       ├── journal/
│   │       │   ├── mod.rs
│   │       │   ├── recorder.rs       # ═══ Codex 模式: async channel ═══
│   │       │   ├── event.rs          # RunEvent 类型
│   │       │   ├── chain.rs          # SHA-256 hash chain
│   │       │   └── verifier.rs       # ═══ Gemini 模式: JournalInvariants ═══
│   │       ├── session/
│   │       │   ├── mod.rs
│   │       │   ├── trust.rs
│   │       │   ├── envelope.rs
│   │       │   ├── lifecycle.rs
│   │       │   └── snapshot.rs       # ═══ OpenCode 模式: shadow Git ═══
│   │       ├── sandbox/
│   │       │   ├── mod.rs
│   │       │   └── job_object.rs
│   │       ├── credential/
│   │       │   └── mod.rs
│   │       └── permit/
│   │           └── mod.rs
│   │
│   ├── xai-grok-shell/               # ← KEEP (modify)
│   ├── xai-grok-tools/               # ← KEEP
│   ├── xai-grok-workspace/           # ← KEEP
│   ├── xai-grok-sandbox/             # ← KEEP
│   ├── xai-grok-mcp/                 # ← KEEP
│   ├── xai-grok-config/              # ← KEEP
│   ├── xai-grok-markdown/            # ← KEEP
│   └── ... (other kept crates)
│
├── schemas/                          # JSON Schema（从 Python 迁移）
│   └── ...
├── fixtures/                         # positive/negative
└── tests/                            # 集成测试
```

## 4. 注入点（从 v0.2，不变但补充细节）

### 4.1 HTTP transport
```rust
// 在 Grok shell 的 Chat Completions 请求构造点:
body["thinking"] = json!({"type": "disabled"});
// 从此删除 scripts/deepseek_thinking_proxy.py
```

### 4.2 Prompt pipeline（事件驱动触发）
```rust
// orz-assurance::orientation::trigger
pub struct OrientationTrigger {
    pub turn_output_tokens: u64,
    pub turn_action_count: u64,
    pub distinct_tool_types: HashSet<String>,
    pub conversation_turn: u64,
    pub cooldown_remaining: u64,
}

impl OrientationTrigger {
    pub fn should_fire(&self) -> bool {
        // 轮次触发 — 不受 cooldown
        if self.conversation_turn > 0 && self.conversation_turn % 8 == 0 {
            return true;
        }
        if self.cooldown_remaining > 0 { return false; }
        self.turn_output_tokens >= 7000
            || self.turn_action_count >= 10
            || self.distinct_tool_types.len() >= 7
    }
}
```

### 4.3 Tool dispatch
```rust
// 执行前: IPG
let ipg = orz_assurance::gates::ipg::evaluate(&ctx, &tool_call)?;
// 执行后: 更新 trigger 计数器
trigger.record_action();
trigger.record_tokens(output.len());
trigger.record_tool_type(&tool_call.tool_id);
// 检索工具完成后: sufficiency check
if tool_call.is_retrieval {
    prompt.push(orz_assurance::orientation::sufficiency::build(sources, ...));
}
```

### 4.4 Session lifecycle + Journal recording
```rust
// 使用 Codex 模式的 async recorder:
let recorder = JournalRecorder::new(journal_path)?;
recorder.record(RunEvent::run_preflight(...));
// ... 运行中 ...
recorder.record(RunEvent::run_finished(...));
recorder.persist()?;

// 如果需要 checkpoint 前快照（OpenCode 模式）:
let snapshot_hash = snapshot_store.track()?;
journal.record(RunEvent::snapshot_created(snapshot_hash));
```

## 5. 统一设计语言（更新）

### 5.1 命名约定

| 概念 | Rust 路径 | 来源模式 |
|------|----------|---------|
| Journal recorder | `orz_assurance::journal::recorder` | Codex `RolloutRecorder` |
| Journal invariants | `orz_assurance::journal::verifier::JournalInvariants` | Gemini `checkContextInvariants` + orz hash chain |
| Shadow Git snapshot | `orz_assurance::session::snapshot` | OpenCode `snapshot/index.ts` |
| Security finding | `orz_assurance::gates::SecurityFinding` | Goose `security_inspector.rs` |
| Permission decision | `orz_assurance::gates::PermissionDecision` | Goose 原则 |
| Orientation trigger | `orz_assurance::orientation::trigger` | 新设计（事件驱动） |
| Info sufficiency | `orz_assurance::orientation::sufficiency` | 新设计 |
| Gate trait | `orz_assurance::gates::AssuranceGate` | 统一 gate 接口 |

### 5.2 Invariants（编译时/启动时机械检查）

1. 每个 action 恰好一个 terminal event
2. Journal hash chain 连续不可断裂
3. `run_finished` 必须是最后一个 event
4. 凭据不写入 journal（只允许 SHA-256）
5. Hidden reasoning 不进入公开输出
6. Orientation checkpoint 不生成 counterexample_candidate
7. 用户授权可满足 permission，不能证明 provenance/coverage/independence
8. **SecurityFinding 不自动产生 PermissionDecision**（Goose 原则）
9. **Snapshot restore 必须写入 journal event**（OpenCode + orz 约束）
10. **Journal event IDs 不重复、turn_id 不孤儿、时间戳不非线性**（Gemini 模式 + hash chain）

## 6. 迁移路线（4 Phase，更新）

### Phase 1: Scaffold + Build
- Fork Grok Build 源码（`500129c7`）
- 建立 `orz-bin/`、`orz-assurance/`（空骨架）、`orz-tui/`（空骨架）
- 更新 workspace `Cargo.toml`，重命名所有 gsa→orz
- 移除 telemetry/cloud/auth/dead crates
- **验证**: `orz --version`

### Phase 2: Journal + Snapshot（注：snapshot 提前到 Phase 2）
- 实现 `orz-assurance::journal::recorder`（Codex 模式 async channel）
- 实现 `orz-assurance::journal::event`（RunEvent 类型）
- 实现 `orz-assurance::journal::chain`（hash chain）
- 实现 `orz-assurance::journal::verifier`（Gemini 模式 invariants + hash chain check）
- 实现 `orz-assurance::session::snapshot`（OpenCode 模式 shadow Git）
- 注入点 1: HTTP transport `thinking: disabled`
- 注入点 4: session lifecycle journal events
- **验证**: `orz -p "hello"` → valid `events.jsonl`, invariants clean, snapshot track/restore 可用

### Phase 3: Gates + Orientation
- 实现 `orz-assurance::gates`（ipg, tool_availability, source_visibility, adapter）
- 实现 `orz-assurance::orientation`（checkpoint, stagnation, sufficiency, trigger）
- 注入点 2 + 3: prompt + tool dispatch
- 实现事件驱动触发
- **验证**: 完整 gate 链通过

### Phase 4: TUI + Polish
- 移植 GSA TUI → `orz-tui`
- 实现 session（trust, envelope, lifecycle）
- 实现 sandbox::job_object, credential, permit
- Python 项目标记为 `reference-spec`
- **验证**: `orz` 全功能通过 Python conformance suite

## 7. Python 项目的新角色

| 角色 | 说明 |
|------|------|
| **Spec reference** | Python 是 Rust 的行为"金版" |
| **Conformance suite** | Rust `orz` 必须通过相同测试 |
| **Design docs** | 架构、协议、ADR、偏差记录 |
| **Rapid prototype** | 新 gate 先 Python 验证再移植 |
| **Schema authority** | JSON Schema 规范源 |

## 8. 与 v0.2 的差异

| | v0.2 | v0.3 |
|---|---|---|
| 产品名 | GSA | **orz** |
| 外部借鉴 | 仅 Grok 结构分析 | **五源融合**：Codex/Gemini/OpenCode/Goose + Grok |
| Journal 写入 | 同步 JSONL append | **Codex 模式** async channel + deferred persist |
| Journal 验证 | hash chain only | **Gemini 模式** 4 维 invariants（dup/orphan/ts/hash） |
| Snapshot | 未设计 | **OpenCode 模式** shadow Git + alternates + semaphore lock |
| Security/Permission | 合并 | **Goose 原则** Finding 与 Decision 分离 |
| Snapshot 位置 | 无 | Phase 2（从 Phase 3 提前） |

## 9. 参考

- Codex CLI: `codex-rs/rollout/src/recorder.rs`, `ordinal.rs` (Apache-2.0)
- Gemini CLI: `packages/core/src/context/utils/invariantChecker.ts` (Apache-2.0)
- OpenCode: `packages/opencode/src/snapshot/index.ts` (MIT)
- Goose: `crates/goose/src/security/security_inspector.rs` (Apache-2.0)
- Grok Build: `xai-grok-shell`, `xai-grok-tools`, `xai-grok-workspace` (Apache-2.0)
- Aider: architect/editor separation, repo map (Apache-2.0)
- 本仓库既有多源审计: [`OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md`](OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md)
- 本仓库职责边界: [`UPSTREAM_FIRST_INTEGRATION_v0.1.md`](UPSTREAM_FIRST_INTEGRATION_v0.1.md)
- 本仓库 ADR: [`ADR-0003`](../adr/ADR-0003-runtime-neutral-assurance-kernel.md)
