# 任务 D S2c 其余 24 族 Rust conformance 实现（2026-09-06）

> **审计日期**：2026-09-06
> **性质**：任务 D（双实现终局治理）S2c 实施批——在 Rust `orz-assurance`
> 为 S2a 盘点表 S2c 三个子批（检索族 8 / 上下文与压缩族 7 / 控制面族 9，
> 共 24 族，含 4 个 B 族）补齐机械规则族校验 + fixture 正/负对拍。验收
> （排期文档 §2.1）：orz-assurance 测试绿 + 与 Python 同族对拍 0 差。
> **范围**：orz 子模块 `195c71b8`（feat/fusion-architecture，3 文件
> 4618+/194-）+ 父仓库登记（manifest 1438 条重算、BACKLOG/TODO 勾选）。
> **关联前序**：[S2a 盘点表](TASK_D_S2A_INVENTORY_2026-09-06.md) /
> [S2b 实施审计](TASK_D_S2B_FAMILIES_IMPL_AUDIT_2026-09-06.md) /
> [S2b 复审处理](TASK_D_S2B_REVIEW_HANDLING_2026-09-06.md)。

## 1. 实现

### 1.1 新增 `orz-assurance/src/journal/families_s2c.rs`（24 个校验器）

逐条子规则镜像 Python `assurance/run_event_journal_validation.py` 的
`_verify_v02_*`（单次顺序扫描 + per-run/per-activation 状态机 + 机械复算；
canonical 字节经 batch-1 `chain::canonical_json`）。

**S2c-1 检索族（8）**

| 族 | 锁定的子规则要点 |
|---|---|
| `result_consistency` | source_counts=ledger 机械分布（四可见性 + total）；highest_allowed_claim 可见性投影矩阵；organized_response 退役；visibility_degraded ⇔ 无文本级证据；result_digest=四段 canonical SHA-256、ledger_digest=source_ledger 摘要、result_id 前缀 `RET-RES-{d16}-`；同 (activation, revision) 在后 assessment 绑定 digest/counts/`no_fulltext_evidence`/`ASSESS-{d16}-` 前缀 |
| `reason_codes` | 词表封闭 2 码；退役码永不得再现 |
| `source_weighting` | web_page 必带 tier；tier/weight/weight_reason 成套；固定乘数表（1.1/1.0/0.7，int==float 数值等值）；退役模型标注字段缺席 |
| `search_candidate_pool` | candidate_urls 仅 web_search_result；非空唯一串列表；空池须有移除留痕；pool 与 urls 同长同序、逐项 canonical_url/tier/weight/relevance/form_reasons 校验；raw_source_refs 双向镜像 |
| `candidate_prefilter` | 有池必有 log；log 引用带池条目；(source,url,reason) 唯一；reason 封闭 5 码 + action=removed；非 duplicate 移除者不得留在保留池 |
| `candidate_count` | count/cap 成对且仅候选计数工具；0≤count≤cap、cap≥1；非 error 车道完成必带（无 target）、包装完成不带；cap 超限拒绝 error+count==cap |
| `receipt_event_isomorphism`（B） | gate 拒绝完成（policy_denial 标记或 `is_denial_code` 结构化码）start 可无但不可在后；非 gate 错误完成 1:1 前置 tool_started；同 call 至多一条 completed；非 run_invalidated 终止的 run 无开放 start（墙钟超时豁免） |
| `probe_accuracy`（B） | 兼容边界：有 request_header_change 才启用；complete 集翻转后必须随主车道 header change（下一 model_output 前） |

**S2c-2 上下文与压缩族（7，全 A）**

| 族 | 锁定的子规则要点 |
|---|---|
| `recovery_truncation` | 先于首个 model_request；rounds_dropped>0 |
| `context_compressed` | mode/reason 枚举；guard_failed 不随 session_end；mechanical 永不 incomplete；incomplete⇔三 null；archive_write_failed 只随 complete |
| `activation_restore` | 同一 activation_id 至多恢复一次 |
| `dep_graph_events` | kind∈{read,write}、path 非空、exit 0；read⇒read_file；write⇒search_replace 且 consumed_read 引用同 run 更早同路径 read、anchor 匹配（sha256 权威 / size+mtime 回退）；anchor/new_anchor 对象或 null |
| `tool_running` | mid-run：started<running<completed、每 call 至多一条、completed 带 running:true 且 exit null、恰一条 completed；idle-kill：前置 mid-run+running completion、晚于 completion、每 call 至多一条、reason 非空 |
| `output_truncation` | truncated⇒total_bytes；object_id⇒truncated+total_bytes 同在 |
| `budget_cue_injected` | 每 run ≤4 条；remaining 严格低于 threshold（600/300/120） |

**S2c-3 控制面族（9，7A+2B）**

| 族 | 锁定的子规则要点 |
|---|---|
| `inquiry_kind` | orientation_checkpoint 的 payload.inquiry_kind == event_type |
| `plan_write` | refill 仅 attempt=1 且后随第二次写且 valid=false；validation_failed_after_refill 仅 attempt=2；机械降级 valid=true、validation 族 valid=false；accepted⇒valid 且 attempt∈{1,2} |
| `console_mode_transition` | c→d / d→c / stay 固定字段组；d→c related 指向同 run 在先 c→d；每 run 一次 streak 询问；transition_id 同 run 唯一；direct 工具事件（含 tool_running，S5-2 P2-3）携带当前 transition_id |
| `console_order_written`（B） | 机械盖章齐备（action 非空、step_id null/非空）；order_id 同 run 唯一；在先同 run `blackboard_action_write` exit 0 完成经 write_call_id 背书、一写至多一单 |
| `console_order_rejected`（B） | phase/step/code 三元组封闭（pre_issue 3 码 / issue 4 步）；reason 非空；同 run written 同 order 且 round/plan_epoch/run_id 戳一致；每 order 至多一条 |
| `mechanical_audit` | kind∈3 枚举；key 非空 / round 非负 int（bool 计入，镜像 Python isinstance）/ summary 非空 / anomaly 可空字符串 |
| `tool_availability_probe` | complete/incomplete 恰好划分 23 工作工具各一次；incomplete reason 中性固定、禁判断词 |
| `request_header` | per-role 链：initial 无 previous/change_kind；change 紧随 last、previous==last、header 不同、change_kind==组件 digest 差异复算（1 个→名，≥2→multiple）；tools 唯一且 tool_count 一致 |
| `inject_budget` | `round_inject_budget_exceeded` 必带 used(≥0)+budget(≥1)；两字段成对且仅该 code 合法 |

### 1.2 接线与门控

- `families.rs` 派发扩展：`verify_family` 路由 31 族；新增
  `S2C_FAMILIES` / `ALL_FAMILIES`（后者 = Python `validate_journal_text`
  调用序，Py 3677-3707）；`verify_all_families` 改按 `ALL_FAMILIES` 遍历。
  conformance 第 5 阶段接线不变（payload-valid 前提下
  `verify_all_families`），家族数量 7 → 31 自动生效。
- **V01 门控口径**：`candidate_count` / `inject_budget` 与 S2b 的
  `policy_denial` / `failure_target` 同样**不过滤 `_is_v02`**（Python 仅
  匹配 event_type），其余 27 族逐事件过滤——与 Python 完全一致；v0.1 工具
  完成面 schema 定义了相关字段，V01 刊照常核验（S2b 复审 P1 口径延续）。

### 1.3 语义保真要点

- Python 值相等语义（`==`）以 `py_value_eq` 复现：数值跨 int/float 比较、
  bool 按 int 强转（True==1）、对象递归无序比较；Python `.get()` 的
  missing/explicit-null 等同以 `py_none` 归一。
- `result_consistency` / `probe_accuracy` 等处的真值判断以 `py_truthy`
  复现；`payload_sha256` = canonical 字节 SHA-256（与 Python
  `_canonical_bytes`+`_sha256_hex` 同构）；gate 判定复用生产谓词
  `crate::lif::channels::is_denial_code`（与 Python `_is_denial_code_v02`
  同表）。
- **B 族按 Python 现行语义镜像**：`console_order_written` /
  `console_order_rejected` 的「rejected 须有在先 written」规则在写单面
  退役后与活动事件面存在真实分歧（恢复/残留订单可发 rejected 而无
  written）——conformance 忠实镜像 Python（含「written 全刊查找、不查
  顺序」的实现细节），目标语义裁决（放宽 vs 生产收口）留 S2d registry
  翻转前与用户决断。
- `receipt_event_isomorphism` 的 run 级配对独立实现，S2a #1 注记的
  `in_flight_tools` 孤儿补事件机制属生产侧构造，本批不重复实现。
- 旧行号区间快照的现场回验：`console_mode_transition` 的
  `tool_running` 盖章口径（S5-2 审查 P2-3）与 `console_order_written`
  的 write_call_id 背书口径（F1）均已按现场 Python 源码锁定。

## 2. 测试与验证

| 验证 | 结果 |
|---|---|
| 表驱动正/负单测 `family_verdicts_match_spec_table`（**172 场景**（60 旧 + 112 新）× **31 族 = 5332 裁决格**，覆盖每族正例 + 全部违规类 + v0.1 no-op/V01 双族执行 + 跨族真值格 + 消息/重放/边界） | ✅ 1 passed |
| **Rust↔Python 逐族对拍** `s2b_family_verdicts_match_python`（同一语料跑双侧：**172 合成场景 + 12 个 v0.2 + 6 个 v0.1 fixture 期刊 = 190 项 × 31 族 = 5890 裁决格 0 差**；Python 侧经 `-X utf8` 子进程逐族调用 31 个 `_verify_v02_*` 原函数；语料精确核算 + 场景 ≥120 / fixture ≥18 下限守卫） | ✅ 1 passed |
| fixture 全量正验（batch-1 既有 7 测试，家族阶段现含 31 族——真实期刊无误报） | ✅ 7 passed |
| 端到端 fixture 家族负测 `family_stage_tamper_detected_end_to_end`（第 5 阶段门控接线回归） | ✅ 1 passed |
| orz-assurance 全量（lib 204 + 集成 8/9/1） | ✅ 0 failed |
| `cargo check --workspace` | ✅ 零警告零错误 |
| `cargo fmt --check` / clippy | ✅ 净（families_s2c 零告警；lif/mod.rs 存量 impl-derivable 告警不变，S2b 审计已登记） |
| 仓库门禁 `check_repository.py`（manifest 重算 1437 → 1438 条） | ✅ Exit 0 |

**场景库维护注记**（对拍与表测试驱动出的两处旧语料勘正，裁决不变）：

1. 两个 retrieval_mode 旧场景的最小 `retrieval_result_committed` payload
   （仅 `activation_id`）升级为 family-shaped 完整构造（`committed()`
   helper）——Python 家族函数直下标 schema 必填字段，非 payload-valid
   语料会使其崩溃（对拍只跑 payload-valid 语义，与 conformance 门控一致）；
   retrieval_mode 族断言不受影响。
2. `ledger_advance_window_reset_ok` 的裸 `context_compressed` 标记事件被
   S2c context_compressed 族在双侧判官上如实标记——登记为跨族真值格。
3. `console_order_rejected_*` 四场景补背书 `blackboard_action_write`
   完成（rejected_ok 等场景在 written 族下需合法 written 链），保持
   单族隔离。

## 3. 边界与后续

- 本批不动 Python 法官、不改事件生产者；A 族定位 = 独立 journal 复核器
  （复算生产构造保证的跨事件/时序不变量）；B 族 = Python 现行语义镜像。
- 已知维持项（对拍口径内）：Python 直下标依赖 payload-valid 前提，Rust
  侧安全取值在该前提下等价；消息文本为 Rust 形态（对拍按每族
  0 vs >0 裁决）；`request_header` 的 role 键以空串归并缺失 role（仅在
  payload-invalid 下可区分，门控后不可达）。
- **S2d**：31 族全量对拍矩阵落盘 + registry 翻转准备（本批
  `verify_family` / `ALL_FAMILIES` API 与对拍模式直接复用）；翻转前需
  裁决 `console_order_written/rejected` 写单面退役后的目标语义。
- S3：门禁真实 fixture journal 校验改接 Rust 法官；S4：Python 双法官
  退役/归档登记。
