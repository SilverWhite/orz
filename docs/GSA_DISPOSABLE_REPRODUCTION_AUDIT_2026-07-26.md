# GSA disposable reproduction 审计（2026-07-26）

## 裁决

通用科学保障已新增首个 no-model disposable reproduction 纵向切片：

- `disposable-reproduction-manifest-v0.1.schema.json` 冻结 source bundle、输入快照、唯一允许动作、输出 artifact 和证据边界；
- `disposable_reproduction.py` 只执行内置 `explicit_euler_decay_replay_v0.1`，不调用外部代码、模型、网络或子进程；
- replay 输出只能写入 source root 之外的 disposable output root，且拒绝覆盖已有 artifact；
- receipt 和独立 verification 都声明 `no_new_independent_evidence` 与 `no_claim_promotion`。
- run-proof 层进一步绑定当前 replay implementation 文件摘要、Python/平台环境摘要、source bundle/input snapshot、
  output receipt、verification digest 和三事件 metadata-only hash chain。
- runtime-preflight projection 将 run-proof 投影为 `runtime/run-manifest-v0.1.schema.json` 与三条
  `runtime/run-event-v0.1.schema.json` 事件，用于证明对接形状，而非执行正式 runner。
- runtime journal 层将三条 projection event 一次性写入 disposable JSONL，并重放验证 schema、serialization digest、
  payload digest、event digest、hash chain 与 terminal event。
- execution lock 层绑定受控源码/schema 文件摘要、git HEAD 观测和 Python 分发版本快照，用于 development
  可重算锁定，而非完整环境复现。
- no-model runner skeleton 层写出 canonical runtime manifest，并逐条 append projection event，随后 replay
  journal 生成 runner receipt。
- runner journal lock/recovery 层在 sidecar advisory lock 内执行 replay→append，并提供 missing-newline/torn-tail
  recovery inspection 与受控 repair/quarantine receipt。
- lifecycle repair policy 层允许 runner resume 在显式 policy 下修复可恢复 journal，再继续 append 剩余事件。

本轮证明的是同一 development fixture 的固定十进制 replay 能重新生成并校验两个 synthetic numerical artifact。
它不证明原始实验独立、科学结论正确、模型能力、正式 EvaluationRunner、holdout 或生产 hard gate。

## 机械边界

manifest 绑定 `computational_decay` review bundle、三份输入 source snapshot 和两个原 artifact。runner 先复用既有
`review_general_science_bundle`，确保 source bundle 仍通过只读审查，再读取 completed action manifest，
最后用 manifest 中的十进制参数复放 explicit-Euler decay：

- `dt=0.1` 输出 `step_count=10`、`final_value=0.3486784401`；
- `dt=0.05` 输出 `step_count=20`、`final_value=0.3584859224085422`；
- 两个输出都按 `GSAS_NUMERICAL_SINGLE_RUN_0_1` 重新验证；
- `/run/step_size`、`/run/step_count`、`/run/final_value`、`/run/final_absolute_error` 必须与原 artifact 对账。

runner 使用 `Decimal(str(step_size))` 固定十进制步长语义，避免二进制浮点尾差成为不可解释的证据差异。
输出文件通过原子 JSON 写入生成；source bundle digest 在 replay 前后复核不变。

## 正反例

新增回归覆盖：

1. 正例：replay 只写 disposable output root，源 fixture 摘要不变，receipt 和 verification 均通过；
2. output root 位于 source root 内时 fail closed；
3. source snapshot digest 被篡改时在写输出前 fail closed；
4. replay 输出被篡改时 independent verifier fail closed；
5. 第二次 replay 试图覆盖已有输出时 fail closed。
6. runtime preflight projection 可生成 schema-valid manifest/events，并检测 manifest/event tamper；
7. disposable runtime journal 可一次性写入 JSONL 并重放，检测 journal tamper、receipt tamper 和覆盖尝试。
8. execution lock 可绑定 proof/projection/journal/source/environment，并检测 source digest tamper 与 journal tamper。
9. no-model runner skeleton 可写出 manifest/journal/receipt，并检测 runner journal tamper、receipt tamper 和覆盖尝试。
10. runner journal append 会创建 sidecar lock；in-progress journal 可在 `require_terminal=false` 下重放；torn tail
    可被只读 recovery inspection 分类。
11. runner journal repair 可规范化 missing newline、quarantine torn tail，并拒绝 history corruption。
12. runner lifecycle resume 在显式 repair policy 下可修复 torn tail 并继续完成 terminal event；无 policy 或
    history corruption 时 fail closed。

仓库检查器独立验证 manifest schema、source bundle digest、input snapshot digest、output ID/path 唯一性、
output artifact schema 绑定、step size pointer、固定 implementation、安全字段和证据边界。

## Run-Proof 联合证明

`build_disposable_reproduction_run_proof` 在 receipt verification 通过后构造第三层证明：

- `code` 记录 `assurance/disposable_reproduction.py` 的 SHA-256 和固定 implementation ID；
- `environment` 记录 Python 版本、实现、平台系统、平台 release 和 filesystem encoding；
- `inputs` 同时包含 source bundle 与三份 input snapshot digest；
- `outputs` 只保留 output ID/path/digest、原 artifact ID 和 artifact schema ID；
- `journal` 固定三条 metadata-only 事件：`run_preflight → run_started → run_finished`，每条事件用 RFC8785
  canonical payload digest 和 previous-event hash 串联。

`verify_disposable_reproduction_run_proof` 重新验证 receipt、重建 proof，并用 canonical bytes 精确比较。新增反例覆盖
journal hash-chain tamper 与 code digest tamper。该 proof 仍不是正式 EvaluationRunner journal：它没有 adapter
preflight、模型事件、外部代码执行、评分交接、失败 taxonomy 或 multi-run lifecycle。

## Runtime Preflight Projection

`runtime_preflight.py` 新增 `build_gsa_runtime_preflight_projection` 和
`verify_gsa_runtime_preflight_projection`。它先重新验证 disposable run-proof，再生成：

- development-mode immutable runtime manifest，`provider=none`、`model_id=no-model`、tool allowlist 为空、
  network disabled、memory disabled；
- 三条 runtime event：`run_preflight`、`run_started`、`run_finished`；
- 每条 event 通过 runtime schema、payload digest 和 event hash 校验；
- projection checks 固定 `no_model_or_tool_events=true` 与 `formal_runner_not_claimed=true`。

该 projection 的 purpose 是让 GSA 本地 proof 能进入 runtime manifest/event 的形状门禁。它仍不包含模型 adapter
preflight、真实 tool broker、结构化评分、failure taxonomy 或 holdout 隔离。

## Disposable Runtime Journal

`write_gsa_runtime_preflight_journal` 将 projection 中的三条 runtime event 以稳定 JSONL 序列化写入目标 journal。
写入使用 exclusive create，已有文件直接 fail closed；receipt 绑定 projection digest、run manifest digest、journal
digest、首尾 event digest、event count 和 terminal event。

`verify_gsa_runtime_preflight_journal` 重新读取 JSONL，逐行解析 runtime event，并检查：

- journal events 与 projection events canonical 相同；
- 文件字节 digest 与 receipt 记录一致；
- run manifest digest、payload digest、event digest 和 previous-event chain 均可重算；
- 全部 redaction 为 `metadata_only`，且不存在 model/tool event；
- 末尾为 `run_finished`，且 `formal_runner_claimed=false`。

该 journal 只是 disposable write/replay 夹具：它没有并发生产 append 临界区、adapter lifecycle、工具 broker、
评分交接或 failure taxonomy，不提升 claim 强度。

## Execution Lock

`execution_lock.py` 新增 `build_disposable_reproduction_execution_lock` 和
`verify_disposable_reproduction_execution_lock`。它在 journal replay 通过后记录：

- `assurance/disposable_reproduction.py`、`assurance/runtime_preflight.py`、`assurance/execution_lock.py`、相关 schema、
  `runtime/run-manifest-v0.1.schema.json` 和 `runtime/run-event-v0.1.schema.json` 的 SHA-256；
- selected source tree 的 canonical digest；
- 直接从 `.git/HEAD` 与 ref 文件读取的 git HEAD 观测；
- Python 版本、实现、平台、filesystem encoding、`jsonschema` 和 `rfc8785` 分发版本。

该 lock 明确声明 `dependency_lock_status=python_distribution_snapshot_only`。它不是完整环境锁，不声明 package
resolver、wheel hash、OS image、container image 或 clean worktree。

## No-Model Runner Skeleton

`runner.py` 新增 `run_gsa_no_model_runner_skeleton`、`replay_gsa_runner_journal` 和
`verify_gsa_no_model_runner_skeleton`。runner 先验证 proof、projection 与 execution lock，再执行：

- 将 projection 的 runtime manifest 写为 RFC8785 canonical bytes，使文件 SHA-256 等于 event 绑定的
  `run_manifest_sha256`；
- 将 projection 的三条 runtime event 逐条 append 到 runner journal；
- 每次 append 都在 sidecar advisory lock 内 replay 当前 journal，检查 sequence、previous-event hash、manifest digest
  与 terminal 状态；
- 完成后 replay 完整 journal，要求唯一 terminal event 为 `run_finished`；
- receipt 绑定 proof/projection/execution lock digest、manifest path/digest、journal path/digest、event count 和末尾 event digest。

该 skeleton 仍是 no-model：不接 adapter、tool broker、外部代码执行、评分、failure taxonomy 或 holdout。

## Runner Journal Recovery

`journal_lock.py` 提供 Windows/POSIX sidecar advisory lock。`inspect_gsa_runner_journal_recovery` 在锁内只读 journal
bytes，并分类：

- `valid_journal`：完整 journal 可直接 replay；
- `missing_newline_normalized`：最后一行是完整 JSON event，仅缺最终换行；
- `torn_tail_removed`：最后一个换行后的尾部损坏，但此前 prefix 可独立 replay；
- `empty_journal`、`no_valid_prefix`、`history_corruption`：不可恢复或需要人工处理。

`repair_gsa_runner_journal` 只接受 `missing_newline_normalized` 与 `torn_tail_removed`：

- missing newline：在锁内写回规范化 journal，并生成 recovery receipt；
- torn tail：必须显式提供 quarantine path，先保存 discarded tail，再写回 retained prefix；
- history corruption、empty journal、no valid prefix 一律拒绝修复；
- 修复后重新 replay journal，失败则恢复原始 bytes 并 fail closed。

当前 repair 不追加 recovery event；它只证明机械 prefix 连续性和修复写回 provenance，不证明 event 语义真实。

## Lifecycle Repair Policy

`gsa-runner-lifecycle-repair-policy-v0.1.schema.json` 定义显式 lifecycle repair policy。`resume_gsa_no_model_runner_skeleton`
只在已有 runner root 上运行：它先验证 manifest 与 projection 匹配，再 replay 当前 journal。若 journal 无法 replay：

- 无 policy：拒绝继续；
- `inspect_only`：拒绝修复；
- `repair_recoverable` 且 classification 被允许：调用锁内 repair，生成 recovery receipt/quarantine，然后继续 append
  尚未写入的 projection events；
- `history_corruption`、`empty_journal`、`no_valid_prefix`：即使有 policy 也拒绝。

该 lifecycle policy 仍不接模型 adapter，也不决定 scoring 或 failure taxonomy。

## 保留缺口

- replay 仍使用同一 source bundle 与原 artifact template；它是复放一致性检查，不增加独立 scientific N；
- 没有真实输入数据、完整 dependency lock、OS/container image 或 clean worktree 证明；当前只锁受控源码和 Python
  分发版本快照；
- 没有 transformation DAG、cycle detection、多 action、多 producer、随机性或数据独立性核算；
- 没有正式 EvaluationRunner 的 adapter preflight、结构化输出校验、评分交接或 failure taxonomy；
- 没有 evaluation/holdout、reviewer-only oracle、human baseline 或阈值校准；
- mechanical PASS 不支持 causal、mechanism、generality 或 production readiness 结论。

## 机械结果

- disposable reproduction 定向测试：31/31；
- 当前源码全量回归：261/261；
- `python scripts/check_repository.py`：129 schemas、1 个 disposable reproduction fixture、0 errors；
- 本轮没有读取或修改 LIF INDEX/MAP/R，也没有登记新的 LIF 科学 claim。
