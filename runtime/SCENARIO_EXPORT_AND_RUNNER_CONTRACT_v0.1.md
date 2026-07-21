# Scenario Export、Leak Scan 与 Runner 契约 v0.1

状态：设计草案；已有 development-only contract probe，无模型运行、无真实 evaluation/holdout 导出。

## 1. 目的

本契约把 corpus 文件变成可被任意模型 adapter 消费、但不携带 oracle 或项目历史答案的运行包，并让每次运行可以从固定 manifest 与 append-only journal 重建。

它定义三个互相隔离的组件：

1. `ScenarioExporter`：确定性裁剪、匿名化和打包；
2. `LeakScanner`：结构、路径和语义泄漏门禁；
3. `EvaluationRunner`：冻结运行条件、调用 adapter、记录事件，不参与改写答案。

## 2. 输入与输出

### Exporter 输入

- corpus 文件及 revision/digest；
- case ID 白名单；
- partition policy；
- fixture manifest schema；
- tokenization seed 或密封 identity map；
- export policy version。

### Exporter 公开输出

- `scenario_bundle/`：被测 Agent 唯一可见内容；
- `scenario-export-manifest.json`：文件列表、digest、策略与 opaque case token；
- `leak-scan-report.json`：`reviewer_only`，不进入场景包；
- append-only export journal。

### 密封输出

- opaque token ↔ internal case ID 的 identity map；
- oracle bundle；
- curation/reviewer fixture 与 source paths。

公开输出与密封输出必须写入不同根目录。软链接、junction、父目录继承读取或模型可访问的绝对路径均视为越界。

## 3. 确定性裁剪

Exporter 必须从每个 case 删除：

- `oracle`；
- `classification`；
- `sources` 及 source path/line；
- `countercase_ids` 与 paired-case identity；
- `curation_fixture_manifest`；
- internal case ID、cluster ID、historical/synthetic 标签；
- revision history 中可能包含的答案说明。

只允许保留：

- task；
- visible facts；
- 明确允许的 `scenario_only` fixture；
- mode、工具/预算边界和输出 schema；
- 无语义的 opaque case token。

`hidden_from_subject` 是 curator 指令，本身也不能原样给 Agent；Exporter 只用它做删除检查。

## 4. Opaque identity

- case token 使用不可表达簇、分区、来源或配对关系的随机值；
- 同一次冻结分区中 token 唯一；
- evaluation 与 holdout 不复用 token；
- bundle 文件顺序在预注册 seed 下随机化；
- identity map 仅在密封维护域存在。

开发模式可保留人类可读 ID 便于调试，但任何结果必须明确标为 development，不能与匿名评测混算。

## 5. Fixture 复制规则

1. 读取 manifest 并通过 schema。
2. `visibility` 必须为 `scenario_only`；`reviewer_only` 立即拒绝。
3. 逐文件验证 SHA-256、相对路径、文件大小和 media type。
4. 拒绝绝对路径、`..`、符号链接/junction、reparse point 和 bundle 根目录逃逸。
5. 不复制 manifest 未登记文件。
6. 复制后重新计算目标文件 digest。
7. 生成确定性 bundle digest；文件排序、路径分隔符和 JSON canonicalization 固定。

Windows 上必须显式检测 NTFS reparse point 和大小写/短路径别名；只做字符串前缀检查不够。

## 6. LeakScanner

Leak scan 分两层。

### 6.1 机械层

至少检查：

- 禁止字段和 JSON pointer；
- reviewer-only fixture/reference；
- internal case/cluster/countercase ID；
- source path、行号和历史轮次文件名；
- known oracle phrases、forbidden claim 原文和 correction summary；
- 未登记文件、hash mismatch、路径逃逸和链接；
- system prompt、memory、检索索引或工具 mount 中是否可访问密封域。

### 6.2 语义层

机械无命中不等于无答案。独立 reviewer 必须判断：

- visible prose 是否直接给出最终 gate/claim；
- 文件名、排序、paired layout 是否暗示结论；
- 合成 fixture 是否用“correct/invalid/expected”等答案性命名；
- task 是否要求复述历史裁决而不是独立分析；
- 删除一段文本后是否仍能从残留纠正叙事反推出 oracle。

模型辅助扫描只能提出候选，不能单独签发 evaluation/holdout pass。

### 6.3 决策

| 模式 | pass | warn/review required | fail |
|---|---|---|---|
| development | allow | allow 并记录 | block export |
| challenge | allow | defer 人工复核 | block export |
| evaluation/holdout | allow | block | block |

Scanner crash、timeout、schema error、未知规则版本或无法读取目标文件，在 evaluation/holdout 一律 fail-closed。该门禁属于内核，不实现为 fail-open hook。

## 7. Immutable RunManifest

每次运行开始前生成不可变 manifest，至少固定：

- scenario export ID、manifest digest 与 bundle digest；
- model provider/model/adapter/version；
- system prompt、project rules 和 generated context digest；
- mode、tool allowlist、permission policy、filesystem/network profile；
- memory、case retrieval、oracle mount、reviewer fixture mount 状态；
- wall time、turn、token 和 tool-call 预算；
- seed、temperature 和 retry policy；
- protocol/reason-code/gate/scoring schema digest；
- journal canonicalization 与 redaction policy。

运行过程中不修改 manifest。任何条件改变都创建新 run ID。

## 8. Adapter 接口

Runner 不依赖供应商消息格式。adapter 至少实现：

```text
capabilities() -> AdapterCapabilities
start(run_manifest, scenario) -> SessionHandle
next(session, normalized_event) -> NormalizedEvent stream
cancel(session, reason) -> CancelReceipt
finalize(session) -> AdapterFinalState
```

`AdapterCapabilities` 必须声明：structured output、tool calling、streaming、seed 支持、context limit、reasoning visibility 和 cancellation。缺失能力不得由 runner 假装存在；应在 preflight 产生 defer/block。

V0 的稳定边界是版本化 JSONL/stdin/stdout。ACP 可作为后续 transport adapter，而不是证据协议本身。

DeepSeek provider 的 thinking/tool transcript、模型解析、兼容层 ignored fields、SSE keep-alive 与重试边界遵循 [`../architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md`](../architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md)。该专项 adapter 只能收紧 preflight 和记录 provider 行为，不能放宽本节的 capability、isolation 或 evidence 边界。

## 9. Tool broker

- tested Agent 只能看到 manifest 允许的工具；
- tool request 先经过内核 gate，再经过 permission policy 和 OS sandbox；
- user approval 只解决 authority，不改变 evidence/independence/provenance 状态；
- evaluation 默认无网络、无项目全盘检索、无 memory、无 historical case retrieval；
- 每次工具调用记录 proposal、permission decision、execution receipt、stdout/stderr digest、artifact refs 与 terminal state；
- 未知 terminal state 不得改写为 success/failure。

## 10. Append-only journal

journal 使用一行一个 `RunEvent` 的 JSONL：

- sequence 从 0 单调递增；
- 每项包含 run ID、event ID、timestamp、event type、payload schema、payload digest 和前一事件 digest；
- payload 采用 RFC 8785 canonical JSON 后计算 SHA-256；
- `event_sha256` 对删除该字段后的完整 event envelope 做 RFC 8785 canonicalization 后计算，下一事件的 `previous_event_sha256` 必须引用它；
- redaction 产生新字段和 redaction record，不覆盖原始事件；
- run finish、failure、cancel 和 invalidation 都是事件；
- summary/index 只是缓存，journal 才是重放来源。

若 journal 出现 sequence gap、digest chain 断裂或 manifest digest 不匹配，attempt 标为 `invalid`，不能只扣分。

`runtime/examples/` 中的零 digest 只验证 schema 形状，不满足上述语义验证，也不能作为真实 journal。

## 11. Runner 状态

```text
draft
  -> exported
  -> scanned
  -> prepared
  -> running
  -> succeeded | failed | cancelled | invalid
```

- 只有 leak verdict 为 pass 才能从 `scanned` 进入 evaluation/holdout `prepared`；
- `failed` 表示系统运行失败；`invalid` 表示评测完整性不可成立；
- 模型答案错误是有效 attempt 的评分结果，不应写成 runner failure。

## 12. 与 Grok Build 的关系

借鉴 Grok Build 的 composition root、headless structured output、ACP 可适配性、session journal、tool/workspace 分层和 permission+sandbox 多层防护。

本项目新增或收紧：

- leak gate 为 kernel hard gate；
- evaluation 默认 deny 与无 memory；
- reviewer fixture 物理排除；
- source/claim/evidence 状态独立于工具执行成功；
- journal 同时支持科学 provenance 和回归重放。

## 13. 当前非目标

- 不实现 TUI、dashboard、remote relay、plugin marketplace 或多 Agent。
- 不创建真实 evaluation/holdout。
- 不决定模型 provider。
- 不自动修改 MAP/INDEX。
- 不在本阶段实现真正调用模型或执行工具的 Rust/Python runner；一次性 Python probe 只验证导出、机械扫描、manifest、journal 与 replay 契约。

## 14. Development-only contract probe

`../prototype/` 当前提供一个可丢弃的 Python probe，用于在生产语言和 adapter 决策前暴露契约错误。它具备以下窄能力：

- 从 revision 4 seed corpus 确定性导出 development/challenge 场景，并把内部 case ID 替换为 HMAC 派生 opaque token；
- 只复制 `scenario_only` fixture，验证相对路径、root containment、reparse/link 与 SHA-256；
- 执行结构、内部 ID、来源路径和 bundle digest 的机械 leak scan；独立语义复核未运行时保持 `warn`；
- 在 development 下允许 `warn` 产物用于 smoke，在 challenge 下 `defer`，evaluation/holdout 仍不可由该 probe 创建或放行；
- 生成明确标记 `no-model-smoke` 的 immutable RunManifest、RFC 8785/SHA-256 hash-chain JSONL journal，并验证 replay 与篡改失败。

该 probe 不是 `EvaluationRunner`，其成功只证明当前机械契约在一个 fixture 上可执行，不证明 Agent 正确性、隔离强度或评测有效性。
