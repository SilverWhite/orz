# Adapter Gate 审计（2026-07-28）

## 裁决

新增 Adapter Gate 审计文档，对适配器调用旁路执行 Gate 的四个模块进行独立审计。Adapter Gate 是 Canonical Guarded CLI Gate 序列中位于 Source Visibility Gate 之后、Answer Packet 之前的强制执行点——确保所有模型/工具适配器调用必须通过 Instruction Provenance Gate (IPG) 的前置验证，并完成预检、输出验证和故障分类的全闭环。

本轮为**审计补漏**（此前该 Gate 无独立审计文档，仅由 schema 与源码约束），不新增代码功能。全部四个模块遵循现有 no-model/offline 边界，不调用真实模型、不联网、不读取凭据实体。

## 问题背景

在 Gate 序列中，IPG (GAK-INJ-001) 负责入口级多源指令分类与反注入，Source Visibility Gate 负责引用源的全文可见性检查。但这两个 Gate 通过后，适配器调用本身仍存在多种旁路风险：

1. **绕过 Gate 直接调用适配器**：运行时可能在未持有有效 IPG receipt 的情况下直接发起模型调用，完全绕过 provenance 检查。
2. **Gate 间状态不一致**：IPG 通过但 Gate 链不完整（如跳过了 Tool Availability Gate 或 Source Visibility Gate），导致适配器在信息缺失下执行。
3. **适配器输出未受控**：模型输出可能包含凭据泄漏、违反 source visibility 声明边界、或暴露原始模型内部字段。
4. **故障无结构化分类**：适配器失败时缺少统一的故障分类与恢复策略，导致重试行为不一致。

Adapter Gate 作为**单一强制点 (single enforcement point)**，将所有适配器调用包装在 `enforce_adapter_call` 之下，确保上述风险在机械层面被阻断。

## 新增组件

### 1. Adapter Gate Enforcer (`assurance/adapter_gate.py`)

核心类 `AdapterGateContext`：
- 不可变上下文，将 IPG receipt 与特定适配器调用绑定
- 属性：`gate_decision`、`gate_valid`、`ipg_receipt_sha256`、`ipg_context_sha256`
- 每次调用生成唯一 `enforcement_id`（`AGE-{UUID}` 前缀）

核心函数 `enforce_adapter_call`：
- 接收 `gate_context: AdapterGateContext` 和 `adapter_call: Callable[[], Any]`
- 按顺序检查：
  1. IPG receipt 是否存在
  2. IPG receipt 是否有效（调用 `verify_instruction_provenance_gate_receipt` 重建比对）
  3. Gate decision 是否为 `"allow"`
- 三项全部通过才执行 `adapter_call()`，否则抛出 `AdapterGateBlockedError`
- 返回 enforcement receipt（含完整 checks、bypass 检测、limitations）

独立验证函数 `verify_adapter_gate_enforcement`：
- 重建 IPG receipt 并校验 enforcement receipt 中的 digest 一致性
- 检查 `gate_decision`、`adapter_call_allowed`、`bypass_attempted` 的逻辑一致性

Bypass 攻击夹具 `run_adapter_gate_bypass_fixture`：
- 支持四种 bypass 场景的受控模拟：
  - `attempt_direct_call=True`：无 Gate 上下文直接调用
  - `tamper_receipt=True`：篡改 IPG receipt（注入伪造字段）
  - `use_blocked_gate=True`：使用 decision 为 `block` 的 Gate
  - `replay_old_receipt=True`：重放过期 receipt
- 每种场景记录 `blocked` 状态、`bypass_details` 和 `fixture_type`

### 2. Adapter Preflight (`assurance/adapter_preflight.py`)

函数 `run_adapter_preflight`：
- 在适配器调用前执行机械预检，**不发起网络请求、不读取凭据**
- 五项检查：
  1. **Endpoint 规范化**：通过 `canonicalize_network_endpoint` 验证端点格式
  2. **凭据可读性探针**：检查 `credential_target` 是否已声明（不读取实际值）
  3. **Gate 链完整性**：IPG + Tool Availability + Source Visibility 必须全部评估
  4. **IPG receipt 有效性**：调用方提供 `ipg_receipt_valid` 布尔值
  5. **适配器能力对齐**：结构化输出需求与 output schema 已知状态匹配
- 返回 `preflight_receipt`，含 `preflight_passed`、`checks`、`errors`

独立验证函数 `verify_adapter_preflight`：
- Schema 验证 + 三项逻辑一致性规则
- 检测矛盾状态（如 passed 但有 errors、IPG 有效但 Gate 链不完整）

### 3. Adapter Output Validator (`assurance/adapter_output_validator.py`)

函数 `validate_adapter_output`：
- 五项输出验证：
  1. **Schema 合规**：answer packet 必须符合 `canonical-cli-answer-packet-v0.1.schema.json`
  2. **声明边界**：Source Gate 为 `block` 时，`scientific_claim_strength` 必须为 `"none"`
  3. **凭据泄漏检测**：启发式扫描 6 种密钥模式（`sk-`, `sk-ant-`, `sk-or-`, `Bearer `, `xai-`, `deepseek-`），带上下文片段捕获
  4. **源可见性注释**：Source Gate 有 reference decisions 时，answer 必须有 `source_visibility_summary`
  5. **原始内部字段泄漏**：检测 `raw_response`、`private_reasoning`、`finish_reason_internal`、`api_key` 是否泄漏到公开字段

### 4. Adapter Failure Classifier (`assurance/adapter_failure_classifier.py`)

函数 `classify_adapter_error`：
- 10 个故障类别，按优先级分类：
  - `output_validation_failed=True` → `invalid_output` (retryable)
  - `timeout=True` → `network_timeout` (retryable)
  - `connection_error=True` → `network_unreachable` (not retryable)
  - HTTP 401/403 → `auth_failure` (not retryable)
  - HTTP 429 → `rate_limited` (retryable)
  - HTTP 5xx → `server_error` (retryable)
  - HTTP 400 → 关键字匹配细分：`context_length_exceeded` / `content_filtered` / `model_unavailable`
  - HTTP 404 → `model_unavailable` (not retryable)
  - Exception 类型/消息匹配 → `network_timeout` / `network_unreachable` / `auth_failure`
  - 兜底 → `unknown` (not retryable)
- 每个类别含 `retryable`、`description`、`recovery_action`

函数 `build_failure_recovery_plan`：
- 基于分类结果生成恢复计划
- 指数回退：`delay = base * (2 ** current_retry)`，base 按类别差异化（rate_limited=5s, network_timeout=2s, server_error=1s, invalid_output=0.5s）
- 达到 `max_retries` 上限后返回 `should_retry: false`

## Gate 链位置

Adapter Gate 在 Canonical CLI Gate 序列中的位置：

```
IPG (GAK-INJ-001) → Tool Availability → Orientation Checkpoint → Source Visibility → [Adapter Gate] → Answer Packet → Journal → Verifier
```

Adapter Gate 是模型实际被调用前的最后一道机械门禁。它消费前方所有 Gate 的输出（IPG receipt、source gate receipt），并保护后方 Answer Packet 的完整性。

## 集成影响

### 与 IPG 的关系
- `AdapterGateContext` 直接绑定 IPG receipt，每次适配器调用必须提供有效的 IPG 上下文
- `enforce_adapter_call` 内部调用 `verify_instruction_provenance_gate_receipt` 重建验证
- 篡改 IPG receipt 的任何字段会导致 canonical bytes 不匹配，触发 `AdapterGateBlockedError`

### 与 Source Visibility Gate 的关系
- Output Validator 消费 source gate receipt 的 `decision` 和 `reference_decisions`
- 当 source gate 为 `block` 时强制 `scientific_claim_strength: "none"`
- 当 source gate 有 reference decisions 时要求 answer 包含 visibility annotations

### 与 Canonical CLI 的关系
- `canonical_cli.py` 的 offline `run` 在 source visibility gate 之后、answer packet 生成之前，应经过 preflight → enforce → output validate → classify 的完整链路
- Preflight 的 `gate_chain_complete` 检查要求 IPG + Tool Availability + Source Visibility 三者均已评估

## 文件清单

### 审计覆盖的模块
| 文件 | 行数 | 用途 |
|---|---|---|
| `assurance/adapter_gate.py` | 372 | 核心 Gate 执行器、AdapterGateContext、bypass fixture |
| `assurance/adapter_preflight.py` | 127 | 适配器预检（5 项机械检查） |
| `assurance/adapter_output_validator.py` | 139 | 输出验证（5 项检查含凭据泄漏检测） |
| `assurance/adapter_failure_classifier.py` | 195 | 故障分类（10 个类别）+ 恢复计划 |

### 关联 Schema 文件
| 文件 | 用途 |
|---|---|
| `assurance/adapter-gate-enforcement-receipt-v0.1.schema.json` | enforcement receipt 结构约束 |
| `assurance/adapter-preflight-receipt-v0.1.schema.json` | preflight receipt 结构约束 |
| `assurance/adapter-failure-classification-v0.1.schema.json` | 故障分类结构约束 |
| `assurance/canonical-cli-answer-packet-v0.1.schema.json` | Output Validator 的参考 schema |

### 测试文件
| 文件 | 测试数 | 覆盖范围 |
|---|---|---|
| `assurance/tests/test_adapter_gate_bypass.py` | 17 | enforcement (6) + bypass fixture (5) + endpoint canonicalizer (6) |
| `assurance/tests/test_adapter_integration.py` | 19 | preflight (4) + output validator (5) + failure classifier (8) + e2e (1) + 端点规范化 (1，与 bypass 测试重叠) |

## 测试覆盖

### Adapter Gate Enforcement（6 个测试）
- 有效 Gate 允许适配器调用
- 篡改 receipt 阻止适配器（canonical bytes 不匹配）
- Gate decision 为 block 时阻止调用
- data-only source 不能升级为 routing
- 注入模式检测（含 content_hints）
- 外部内容伪装为用户来源

### Adapter Gate Bypass Fixture（5 个测试）
- 直接调用（无 Gate 上下文）被阻止
- 篡改 receipt 夹具被阻止
- 被阻止的 Gate 夹具被阻止
- 有效夹具允许调用
- Verifier 检测 enforcement receipt 不匹配

### Adapter Preflight（4 个测试）
- 有效配置通过预检
- 缺少 IPG 时预检失败
- 非法端点（ftp://）时预检失败
- Verifier 检测 passed + errors 矛盾

### Adapter Output Validator（5 个测试）
- 有效输出通过全部检查
- 缺少 source visibility 注释时被检测
- 凭据泄漏（sk-ant-api03-...）被检测
- 原始内部字段泄漏被检测
- 声明边界 blocked 时 claim strength 被检查

### Adapter Failure Classifier（8 个测试）
- auth_failure 分类（HTTP 401）
- rate_limited 分类（HTTP 429）
- server_error 分类（HTTP 503）
- network_timeout 分类（timeout=True）
- connection_error 分类（connection_error=True）
- output_validation_failed 分类
- 恢复计划 max_retries 上限
- 恢复计划指数回退递增
- 非 retryable 类别不重试

### End-to-End 集成（1 个测试）
- 完整链路：preflight → gate enforce → adapter call → output validate → classify/plan

### Endpoint Canonicalizer（6 个测试，与 bypass 测试同一文件）
- 路径遍历拒绝
- POSIX 路径返回
- 网络端点规范化（移除默认端口）
- 非 HTTP 协议拒绝（ftp://）
- 端点 allowlist 强制
- 文件系统 blocklist 强制

## 边界与限制

- **Gate 执行是机械性的**：检查 receipt 的存在性和有效性，但无法阻止恶意适配器在获得允许后故意忽略 Gate 结果
- **凭据检测基于启发式**：使用子串模式匹配，可能对合法内容中包含类似模式的文本产生误报
- **Preflight 不发起网络连接**：不检测 provider 宕机、模型弃用或速率限制状态——这些都是运行时错误，由 Failure Classifier 事后处理
- **Source visibility 注释检查是结构性的**：验证注释字段是否存在，不验证其语义正确性或完整性
- **HTTP 400 分类依赖关键字匹配**：provider 可能改变错误消息格式，启发式可能需要更新
- **Replay 检测要求调用方每次提供全新的 gate context**：enforcement receipt 记录 context 身份但自身不维护 nonce 存储
- **当前仍为 offline/fake 模式**：不调用真实模型、不发起网络请求

## Canonical CLI 接入

`assurance/canonical_cli.py` 的 offline `run` 路径应在 source visibility gate 之后执行完整的 Adapter Gate 链：

1. `run_adapter_preflight` — 预检（端点、凭据声明、Gate 链完整性）
2. `enforce_adapter_call` — Gate 强制（IPG receipt 验证 + 适配器调用包装）
3. `validate_adapter_output` — 输出验证（schema、凭据泄漏、声明边界）
4. 若验证失败：`classify_adapter_error` + `build_failure_recovery_plan` — 故障分类与恢复

当前 offline 模式下 adapter_call 为 no-model fixture，不做真实 API 调用。

## 下一步

后续适合接入：
- 真实 runtime adapter 的 hook 注册（替代当前 fixture-only 的 bypass 测试）
- 将 failure classifier 连接到实际的重试循环中（当前 recovery plan 由调用方自行执行）
- 对凭据检测增加语义级验证（如熵检测），降低误报率
- 实现 nonce-backed replay protection（替代当前的 identity-based 方案）
- 增加网络可达性探针作为 preflight 的可选步骤（当前仅做端点规范化）
