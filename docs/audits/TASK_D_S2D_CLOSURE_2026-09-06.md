# 任务 D S2d 收口审计（2026-09-06）

> **性质**：S2d 正题收口——「30 族全量 Python↔Rust 对拍 0 差矩阵落盘 +
> registry 翻转准备」。**本批零代码改动、零生产面改动**：全量对拍为既
> 有已提交测试的新鲜实测（orz `1595303b`，工作树干净），翻转准备为只读
> 触点盘点与方案登记，翻转执行本身属 S3、待用户放行。
> **关联**：[S2d 翻转裁决登记](TASK_D_S2D_FLIP_ADJUDICATION_2026-09-06.md)
> / [批 1 复审处理](TASK_D_S2D_BATCH1_REVIEW_HANDLING_2026-09-06.md) /
> [批 2 复审处理 §4 遗留](TASK_D_S2D_BATCH2_REVIEW_HANDLING_2026-09-06.md)
> / [batch-1 治理审计 §6](P0_GOV_TASK_D_DUAL_IMPL_GOVERNANCE_2026-09-04.md)
> / BACKLOG 00 S2d / TODO P0-GOV S2d。

## 1. 全量对拍矩阵落盘（实测，非转抄）

### 1.1 语料构成（对拍语料 = 251）

| 类别 | 数量 | 明细 |
|---|---|---|
| 合成 spec 场景 | 233 | Rust `families.rs` tests `scenarios()`（S2b 六族 60 + S2c 检索/上下文/控制面族 + S2d 退役口径改写），由计数守卫 `assert_eq!(scenario_count, 233)` 钉死 |
| 真实 fixture（v0.1） | 6 | `runtime/fixtures/run-event-v0.1/journals/`：cancelled-run / failed-run / plain-run / plan-run / restore-run / tool-snapshot-run |
| 真实 fixture（v0.2） | 12 | `runtime/fixtures/run-event-v0.2/journals/`：cancelled-run / cross-prompt-restore / failed-run / local-browser-capability / local-browser-read / mode-off-refusal / orientation-fire-run / plain-run / plan-run / real-doc-retrieval / restore-run / tool-snapshot-run |

### 1.2 30 族清单（对拍族集 = `ALL_FAMILIES`，Py 调用序）

inquiry_kind / plan_write / console_mode_transition / console_order_rejected /
ledger_fold_advance / ledger_fold_write_failed / lifecycle / tool_running /
output_truncation / budget_cue_injected / retrieval_mode / result_consistency /
reason_codes / source_weighting / search_candidate_pool / candidate_prefilter /
candidate_count / inject_budget / policy_denial / failure_target /
receipt_event_isomorphism / dep_graph_events / mechanical_audit /
recovery_truncation / context_compressed / activation_restore / control_tickets /
tool_availability_probe / request_header / probe_accuracy。
（`console_order_written` 已随 S2d 裁决一退役、不在族集，31 → 30。）

### 1.3 矩阵与 0 差实测（2026-09-06 新鲜运行）

| 矩阵 | 维度 | 实测结果 |
|---|---|---|
| **spec 表**（Rust 侧期望裁决格） | 233 场景 × 30 族 = **6990 格** | `family_verdicts_match_spec_table` ✅（186 负例格逐格断言须触发、其余 6804 格断言零触发） |
| **Rust↔Python 对拍**（裁决 parity 格） | 251 语料 × 30 族 = **7530 格** | `s2b_family_verdicts_match_python` ✅ **0 差**（逐格 assert_eq + 精确格数记账守卫） |
| **fixture 侧 Python 判官直扫** | 18 期刊 × 30 族 = **540 族格** + schema 级校验 | schema 级 **0 错误**、族格 **0 firing**（合法期刊零违规，与 Rust 侧 verdict 一致） |

### 1.4 逐族负例覆盖矩阵（spec 表 186 负例格分布，30/30 族全覆盖）

| 族 | 负例格 | 族 | 负例格 |
|---|---|---|---|
| lifecycle | 15 | console_mode_transition | 11 |
| receipt_event_isomorphism | 14 | request_header | 11 |
| result_consistency | 13 | search_candidate_pool | 11 |
| tool_running | 11 | dep_graph_events | 10 |
| candidate_prefilter | 8 | context_compressed | 8 |
| plan_write | 8 | console_order_rejected | 7 |
| retrieval_mode | 7 | failure_target | 6 |
| mechanical_audit | 6 | ledger_fold_advance | 5 |
| policy_denial | 5 | candidate_count | 5 |
| inject_budget | 4 | source_weighting | 4 |
| control_tickets | 3 | tool_availability_probe | 3 |
| budget_cue_injected | 2 | output_truncation | 2 |
| reason_codes | 2 | recovery_truncation | 2 |
| activation_restore | 1 | inquiry_kind | 1 |
| probe_accuracy | 1 | （30 族无一零覆盖） | |

fixture 侧逐族 firing 全 0（18 期刊为合法语料；负例信号全部由合成场景承
载，真实期刊承载「生产合法形态零误报」信号——S2d 两批裁决收窄后新刊与
历史刊在两侧判官下均 0 错误，即批 2 复审 §4 所述 S3 改接前置风险已清除
的再证实）。

### 1.5 判官测试面汇总

| 套件 | 结果 |
|---|---|
| `cargo test -p orz-assurance --lib`（含 spec 表 + 全量对拍两关键测试） | **204 passed / 0 failed** |
| `cargo test -p orz-assurance --test fixture_journal_conformance`（18 期刊过 Rust schema 级法官 + 负篡改） | **9 passed / 0 failed** |
| `python -m pytest runtime/tests/test_run_event_journal_validation.py`（Python 判官） | **258 passed** |
| Python 判官 18 fixture 直扫（§1.3 第三行） | schema 级 0 错误 / 540 族格 0 firing |

## 2. registry 翻转准备（只读盘点，不执行）

### 2.1 现状权威链（翻转前）

1. **权威源**：Python dict `PAYLOAD_SCHEMA_BY_EVENT_TYPE`（v01=34）/
   `PAYLOAD_SCHEMA_BY_EVENT_TYPE_V02`（v02=27）
   （`assurance/run_event_journal_validation.py:49/:95`）。
2. **投影**：`runtime/run-event-payload-registry-v0.1.json`（v01 34 + v02
   27；`source` 字段自证 = 上述 dict；由
   `scripts/export_run_event_payload_registry.py` 导出）。
3. **门禁强同步**：`scripts/check_repository.py:2420-2454` JSON↔dict 同构
   比对，漂移即报错；registry + 导出脚本在 required 文件清单
   （`:2456-2469`）。
4. **门禁 journal 校验**：`check_repository.py:2364`（v0.1，6 期刊）与
   `:2404`（v0.2，12 期刊）两循环共 **18 期刊**调 Python
   `validate_journal_file`（`:3621`）——**双轨全量均在门禁期刊校验内**
   （v0.1 期刊的 payload/envelope 夹具另按实例校验走 `:1948-2010`）。
   *（2026-09-06 S3 深挖批勘误：本节原写「门禁只校 12 v0.2 期刊」漏计
   `:2364` 的 v0.1 循环，实际门禁期刊校验覆盖 18 个双轨期刊，§2.3/§2.4
   相应口径以此为准——v0.1 纳入门禁为既成事实，S3 改接只需 Rust CLI 等
   面承接。）*
5. **Rust 法官**：`orz-assurance/src/journal/conformance.rs:302`
   `validate_journal_file(journal_path, repo_root)`——**库函数入口，无独立
   CLI**；`--repo-root` 参数化已备（batch-1 审计 §6 措辞）；消费点仅
   `fixture_journal_conformance.rs` 集成测试。

### 2.2 S3 翻转内容（既定，batch-1 审计 §6）

1. 门禁真实 fixture journal 校验改接 Rust 法官；
2. Python `validate_journal_file` 对 Rust 轨退役；
3. registry dict 源翻转 JSON 转正（JSON 升唯一权威表）。

### 2.3 前置条件核验（本批实测，全部满足）

- [x] 30 族 Rust conformance 显式实现齐备且对拍 0 差（§1.3，7530 格）。
- [x] 收窄后新刊与 18 历史期刊在两侧判官下 0 错误（§1.3/§1.4；批 1/批 2
  复审遗留的「改接前置风险」就此清除）。
- [x] Rust 法官对 v0.1+v0.2 双轨 18 期刊全通过（fixture_journal_conformance
  9 passed；覆盖面 ≥ 门禁现用 12 v0.2 期刊）。
- [x] registry JSON↔dict 同步门禁绿（门禁复跑 Exit 0，见 §4）。
- [x] orz 工作树干净（`1595303b`），Rust 侧无未提交依赖。

### 2.4 S3 执行步骤序列（建议，每步带验证与回退）

| 步 | 内容 | 验证 | 回退 |
|---|---|---|---|
| 1 | registry 转正：JSON 升唯一权威表（`source` 字段改指 JSON 自身）、Python dict 改由 JSON 派生（模块加载时读取 registry，dict 保留为兼容视图）或直接删除 dict 并改 `check_repository.py` 同步比对方向 | 门禁 registry 计数不变（34+27）；Python 判官测试 258 passed；对拍测试不受影响（判官经 dict/JSON 同表） | 单提交 revert |
| 2 | 门禁期刊校验改接：`check_repository.py:2403-2407` 的 Python `validate_journal_file` 循环替换为 Rust 法官调用（**建议独立 CLI**：orz-bin 增 `orz journal-conformance <journal> --repo-root <root>` 薄封装，门禁逐刊调用；备选 = 门禁调 `cargo test`，但把测试运行时拖入门禁、慢且脆） | 门禁 Exit 0 且 `run_event_v02_journal_fixtures` 计数 12 不变；人为篡改一份 fixture 副本验证门禁能报错（负测后还原） | 单提交 revert |
| 3 | Python Rust 轨退役：`validate_journal_file`（及仅为其服务的轨校验路径）按 S4 口径标注退役/归档，v0.1/v0.2 机械规则族保留至 S4 一并处置 | 门禁 Exit 0；required 文件清单同步 | 单提交 revert |

**S3 待用户裁决点**（登记，不在本批代答）：
- 接线形态：独立 CLI（推荐，见步 2）vs `cargo test` 复用；
- v0.1 期刊是否随改接一并纳入门禁期刊校验（现状门禁只校 12 v0.2 期刊，
  Rust 法官已具备双轨能力、纳入为纯增益）；
- Python 侧退役边界（步 3）与 S4 归档范围的切分。
- S3 为翻转执行批，按本项目惯例**待用户放行后开工**。

## 3. 变更清单

- 新增：本文档。**零代码改动、零 schema 改动、零 registry 改动、orz 子模
  块零提交**（维持 `1595303b`）。
- BACKLOG 00 S2d 勾选收口、TODO P0-GOV S2d 勾选收口（本批同步）。

## 4. 门禁复验

- `python scripts/generate_orz_source_manifest.py`：重算无漂移（orz 零改
  动，1438 条不变）。
- `python scripts/check_repository.py`：**Exit 0**（error_count=0，
  `run_event_payload_registry` 计数 1、registry 无漂移、schemas 246）。

## 5. 遗留与后续

- S3（门禁改接 + registry 转正）：按 §2.4 序列执行，**待用户放行**；
  §2.4 四个裁决点届时一并定案。
- S4（Python 双法官退役/归档登记、契约文档 §8 变更流程同步、索引/BACKLOG
  收口）：S3 闭合后收尾。
- 批 2 复审 §4 登记项（R6 plan-gate 记账面 / R7 回写守卫 / R9②③）不随
  S2d/S3 变动，仍随相应复活/接线裁决议定。
