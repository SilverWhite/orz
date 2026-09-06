# 任务 D S2b 核心六族 Rust conformance 实现（2026-09-06）

> **审计日期**：2026-09-06
> **性质**：任务 D S2b 实施批——在 Rust `orz-assurance` 为 S2a 盘点表定的
> 核心六族（7 个函数入口，全部 A 档）补齐机械规则族校验 + fixture 正/负
> 对拍。验收（排期文档 §2.1）：orz-assurance 测试绿 + 与 Python 同族对拍
> 0 差。
> **范围**：orz 子模块 `809cdb4e`（初版）+ 复审处理批（2026-09-06，P1×2
> 代码修复 + 测试加固，见
> [S2b 复审处理](TASK_D_S2B_REVIEW_HANDLING_2026-09-06.md)）+ 父仓库登记。
> **关联前序**：[S2a 盘点表](TASK_D_S2A_INVENTORY_2026-09-06.md) /
> [S2a 复审处理](TASK_D_S2A_REVIEW_HANDLING_2026-09-06.md) /
> [排期登记](P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。

## 1. 实现

### 1.1 新增 `orz-assurance/src/journal/families.rs`

七个规则族校验器，逐条子规则镜像 Python
`assurance/run_event_journal_validation.py` 的 `_verify_v02_*`（单次顺序
扫描 + per-run/per-activation 状态机 + canonical 字节幂等重放比较，复用
batch-1 `chain::canonical_json`）：

| 族 | 锁定的子规则 |
|---|---|
| `control_tickets` | ①终态事件（consumed/rejected）引用同刊更早 issued 同名 ticket_id（未知票报错）；②issue/终态 ticket_kind 一致；③一票至多一个终态（one-shot） |
| `retrieval_mode` | ①session_bootstrap transition 至多一次；②old_mode 链式等于前一 transition 的 new_mode；③off 窗口内无检索 dispatch（target 命中 + browser_read host 车道）、无 assessment、无 committed result；④local_browser 必带合法 capability_status，非 available 窗口内无 committed result、无非 error 完成 |
| `ledger_fold_advance` | ①fold 点形状（start/cut 非负 int、start<cut、rounds≥1、estimate≥0、role 车道枚举）；②view_estimate_after 非负且严格 < view_estimate_tokens；③per-run 窗口内 fold_start 恒定、fold_cut 严格递增、rounds 不减；④context_compressed 重置窗口 |
| `ledger_fold_write_failed` | ①完整审计形状（path 非空/attempt≥1/disabled bool/rows≥1/estimate≥0/role 枚举）；②attempt 连续 +1、ledger_fold_advance 重置；③disabled==(attempt≥3)（FOLD_WRITE_FAILURE_LIMIT=3）；④耗尽后不得再有 write failure |
| `policy_denial` | ①status=error 且 exit_code 非 0 非 bool；②code 非空、reason 为 string；③source→工具族映射（acaf→票据族 / retrieval_mode→检索面谓词 / permission→work∪host 检索 / taint→work / 未知源报错） |
| `failure_target` | ①仅挂 status=error 完成；②kind 4 枚举；③id 64 位小写 sha256 hex；④kind 专属字段（cmd_preview ≤80 字节 / anchor 三元组含 anchor_hash hex 与 size≥0 非 bool / path 非空 / canonical_url 非空）；⑤kind→工具族映射 |
| `lifecycle` | ①disposition 引用更早 assessment（或 restore 声明）且 CAS revision/activation 一致；②accepted 作用于当前 revision；③每 assessment 一个 accepted 决策；④continue → revision+1 且 assessment 单调不回退；⑤normal_close 绑定 validated disposition（decision/outcome/revision/assessment/activation/contract_id/result_digest）；⑥close 后冻结；⑦重放 payload canonical 字节一致（含 assessment 重放） |

公开 API：`S2B_FAMILIES`（Python 调用序）、`verify_family(name, events)`、
`verify_all_families(events)`（S2d 全量对拍直接复用）。

### 1.2 conformance 第 5 阶段接线

`journal/conformance.rs` 在 payload schema + 链校验后新增：**仅当 payload
无错且轨道为 V02** 时运行 `verify_all_families`——镜像 Python 门控
（`not payload_errors` 才跑跨层检查；V01 刊上每族 `_is_v02` 过滤为 no-op，
Rust 单轨同质下等价跳过）。

### 1.3 语义保真要点

- Python `isinstance(x, int)` 的 bool 兼容（bool 是 int）以 `py_int` 复现；
  `exit_code`/`size` 的显式 bool 排除同样复现。
- 幂等重放比较 = canonical JSON 字节（`chain::canonical_json`，与 Python
  `_canonical_bytes` 同构）。
- B 族注记跟进：S2a 复审处理 #1 指出的 `in_flight_tools` 孤儿补事件机制
  属 `receipt_event_isomorphism`（S2c 范围），本批未重复实现。
- 已知形态边界（与 Python 一致，非偏差）：消息文本为 Rust 形态
  （对拍按「每族报错 0 vs >0」裁决）；Python 侧对 schema 必填字段直接
  下标（其家族仅在 payload 校验通过后运行），Rust 侧安全取值在
  payload-valid 前提下等价。

## 2. 测试与验证

> 数字口径：下表为 **S2b 复审处理批（2026-09-06）后的现场值**。初版登记的
> 「38 场景 / 266 格 / 50 语料 / 350 格 / 集成 17」为勘误（实际 39/273/51/
> 357/16+doctest），并已随复审处理扩容——勘误与修复明细见
> [S2b 复审处理](TASK_D_S2B_REVIEW_HANDLING_2026-09-06.md)。

| 验证 | 结果 |
|---|---|
| 表驱动正/负单测 `family_verdicts_match_spec_table`（**60 场景** × 7 族 = **420 裁决格**，覆盖每族正例 + 全部违规类 + v0.1 no-op + 窗口重置 + 重放幂等/冲突 + close 绑定负路径 + restore 负分支 + pass-through outcome + file/url_target） | ✅ 1 passed |
| **Rust↔Python 逐族对拍** `s2b_family_verdicts_match_python`（同一语料跑双侧：**60 合成场景 + 12 个 v0.2 + 6 个 v0.1 fixture 期刊 = 78 项 × 7 族 = 546 裁决格 0 差**；Python 侧经 `-X utf8` 子进程 import `run_event_journal_validation` 调 `_verify_v02_*` 原函数；语料精确核算 + 场景/fixture 下限守卫） | ✅ 1 passed |
| fixture 全量正验（batch-1 既有 7 测试，现含家族阶段——真实期刊无误报） | ✅ 7 passed |
| **端到端 fixture 家族负测** `family_stage_tamper_detected_end_to_end`（篡改 orientation-fire-run 真实期刊的 disposition CAS 字段 + 链重封 → 经 `validate_journal_file` 断言仅家族阶段报错——锁死第 5 阶段门控接线） | ✅ 1 passed |
| orz-assurance 全量（lib 204 + 集成 18） | ✅ 0 failed |
| `cargo check --workspace` | ✅ 零警告零错误 |
| `cargo fmt --check` / clippy | ✅ 净（families.rs 零告警；lif/reducer 存量告警不变） |

## 3. 边界与后续

- 本批不动 Python 法官、不改事件生产者；A 族定位 = 独立 journal 复核器
  （复核生产构造保证的跨事件/时序不变量）。
- S2c：其余 24 族按档位收口（检索族 8 / 上下文与压缩族 7 / 控制面族 9，
  含 4 个 B 族——其中 console_order_written/rejected 实施前需先裁决写单面
  退役后的目标语义）。
- S2d：全量对拍矩阵落盘（本批 `verify_family` API 与对拍测试模式直接
  推广）；S3：门禁改接。
