# 任务 D S2c 复审处理（2026-09-06）

> **审计日期**：2026-09-06
> **性质**：S2c 实施批（[S2c 实施审计](TASK_D_S2C_FAMILIES_IMPL_AUDIT_2026-09-06.md)，
> orz `195c71b8` / 父 `2323e24`）的三路全面复审处理——语义符合性（Rust↔Python
> 逐族对照）、实现合理性与测试质量、设计合理性与治理登记。修复批 orz
> `a7981654`（4 文件 978+/24-）+ 父仓库登记。
> **方法**：三个独立只读审查视角并行执行，全部发现按 P1/P2/P3 裁决处理；
> 每条裁决附证据（Python/Rust 行号或命令输出）。

## 1. 发现与处置总表

| # | 来源 | 严重度 | 发现 | 处置 |
|---|---|---|---|---|
| 1 | 语义 | **P1** | `dep_graph_events`：`consumed_read=""`（空串）——Python 报 "must be a non-empty string or null" 且不进 lookup（Py 2891-2897）；Rust 以空串查 reads 表，恰命中同 call_id read fact 时静默通过（0 vs >0 真实分歧）。schema 的 `consumed_read` 为 `{"type":["string","null"]}` 无 minLength，payload-valid 可达 | **修复**（orz `a7981654`）：`str_of(consumed).filter(非空)`，空串落入报错分支并 continue（families_s2c.rs dep_graph consumed_read 段） |
| 2 | 语义+实现 | **P2** | 浮点整数字面量（`1.0`/`0.0`）在**值比较**语境的系统性分歧：schema `"integer"` 接受零小数浮点，Python 原生值比较放行（`1.0 == 1`）、Rust `py_int`（isinstance 语义）报错——5 处：recovery_truncation `rounds_dropped`、request_header `tool_count`、plan_write `attempt`、result_consistency `source_counts` 计数与 assessment 键 `contract_revision`（Python dict 键 `hash(1.0)==hash(1)` 归并，Rust 漏配对） | **修复**：新增 `py_int_value`（零小数浮点归一、bool 按 0/1，仅用于值比较语境；`isinstance(int)` 语境保持 `py_int`——candidate_count/inject_budget/mechanical_audit round/console streak 处 Python 浮点本就报错，原实现一致），5 处替换（orz `a7981654`） |
| 3 | 实现 | **P2** | 覆盖缺口：24 族约 100 个 `errors.push` 分支中 55-60 个无场景覆盖（对拍零信号区），主要在 result_consistency 复算分支、search_candidate_pool 形状分支、console_mode_transition 字段组、tool_running idle-kill、request_header change_kind 复算、candidate_prefilter、dep_graph、mechanical_audit、context_compressed、plan_write、console_order_written、receipt gate 顺序 | **修复**：+58 场景（172 → **230**），覆盖上列全部缺口分支；spec 表 **230×31=7130 裁决格**、对拍 **246 语料×31=7626 格 0 差**；场景总数由下限守卫改为**精确钉死 `assert_eq!(230)`**（治理 P3-1 一并收口） |
| 4 | 治理 | **P2** | probe_accuracy 封存工具翻转遗留风险（S2a §3 注记：R1 封存工具翻转不进声明面可产 Python 违规 journal，tool_probe.rs:983）未在 S2c 审计延续登记，S2d 翻转前裁决清单不完整 | **登记修复**：S2c 审计 §3 补边界、本审计 §3 裁决清单、BACKLOG S2c/S2d 行同步（见 §3） |
| 5 | 治理 | **P2** | S2a §6.2 的 11 项 A 档缺口子规则清单未逐项对账 | **对账**（见 §2）：10 项已在 S2b/S2c 场景覆盖，1 项（mechanical_audit plan_gate/budget kind 事件）由覆盖批补齐（ma_plan_gate_ok / ma_budget_ok） |
| 6 | 语义 | P3 | result_consistency assessment 的 `no_fulltext_evidence` 双向检查用派生 no_text_evidence，Python 用声明值 `p["visibility_degraded"]` | **采纳**：改 `py_truthy(visibility_degraded)`（families_s2c.rs） |
| 7 | 实现 | P3 | search_candidate_pool：candidate_urls 存在但非数组时静默跳过（Python 报错）；refs 回环 missing-vs-null 归一 | **采纳**：非数组报错（注意「缺失」仍双侧静默）；refs 比较双测 py_none、presence 检查保持 `is_some`（显式 null 计 present，镜像 `field in ref`） |
| 8 | 实现 | P3 | conformance.rs 第 5 阶段注释过时（"Five families filter"） | **采纳**：更新为四族不过滤口径（policy_denial/failure_target/candidate_count/inject_budget） |
| 9 | 实现 | P3 | e2e 篡改只打 S2b lifecycle 族 | **采纳**：新增 `family_stage_s2c_tamper_detected_end_to_end`——real-doc-retrieval 期刊篡改 `source_counts.total` + 链重封，断言仅 result_consistency 族报错（锁死 S2c 派发在真实门控链路的接线）；fixture_journal_conformance 9 passed |
| 10 | 治理 | P3 | 审计 §1.1 dep_graph 行缺 ④「命令/检索不带 dep_graph」⑤「事实可选」表述 | **登记修复**：§2 对账注记说明两条由「missing-continue + kind→tool 检查」承载（与 Python docstring D3 边界同构），Rust 无语义遗漏 |
| 11 | 语义 | P3（接受） | `py_value_eq` 大整数（≥2^53）跨 int/float 比较精度 | **接受维持**：仅 crafted 大数 stamp 可达，生产者不产 |
| 12 | 语义 | P3（接受） | console_order_written 的 Python `str(payload.get("call_id",""))` 强转（null→"None"） | **接受维持**：仅非字符串 call_id（payload-invalid）分歧 |
| 13 | 语义 | P3（接受） | digest 复算的浮点序列化差异（Python repr `1e-07` vs ryu `1e-7`） | **接受维持**：当前 schema 四段内浮点仅 mechanical_weight ∈ {0.7,1.0,1.1}，两侧序列化相同；schema 放宽时升级处理 |
| 14 | 实现 | P3（接受） | 载荷 `payload.cloned()` 深克隆（性能可省）；`verify_all_families` 错误无族归属前缀；`verify_family` 公开 API 无门控前提说明 | **接受维持/延后**：31 族全线性、数千事件刊无可感受风险；族归属前缀与门控前提 doc 留 S3 门禁改接时一并处理（门禁输出诊断需要时） |

**已核对无偏差项**（审查确认，不需动作）：19/24 族语义逐条一致（含 `_is_v02`
过滤矩阵、状态机键隔离、`is_denial_code` 全表、WORK_TOOLS 23 项、
`ALL_FAMILIES` 31 项调用序与 Py 3677-3707 逐项同序）；panic 审计零
unwrap/expect、`digest16` 切片受 sha256 schema 约束不可达；31 族全 O(n)；
对拍防线（`-X utf8`、精确核算、挂载守卫、双下限）；18 fixture 经 Python
`validate_journal_text` 全量 0 错误（对词语料 payload-valid 确证）；治理
登记全部数字实核属实（204 lib / 集成 9+9+1 / workspace 0 / 门禁 Exit 0 /
manifest 1438 / 提交规模）；INDEX 未更新符合 §0.5；orz-host flaky 无连带。

## 2. S2a §6.2 缺口子规则对账（11 项）

| 缺口项 | 状态 |
|---|---|
| lifecycle 三分支直接测试（stale/conflict/replay-conflict） | ✅ S2b 场景（lifecycle_stale_accepted / lifecycle_cas_mismatch / lifecycle_conflicting_replay） |
| policy_denial 统一形状校验 | ✅ S2b 统一校验器 + 复审修复（permission 精确集合） |
| taint 源测试 | ✅ S2b 场景（policy_denial_taint_ok） |
| failure_target id hex 复核 | ✅ S2b 校验器（is_sha256_hex）+ failure_target_bad_id |
| context_compressed incomplete 退役分支 | ✅ S2c 场景（mechanical_incomplete / incomplete_with_archive） |
| tool_running 顺序断言 | ✅ S2c 校验器 + 场景（started<running<completed 全链） |
| plan_write rotate_failed | ✅ S2c 场景（plan_write_rotate_degrade_valid_false） |
| console_mode_transition 事件级断言 | ✅ S2c 校验器 + 覆盖批（direct 工具盖章、streak、related 链） |
| mechanical_audit plan_gate/budget 断言 | ✅ 覆盖批（ma_plan_gate_ok / ma_budget_ok——本批复审前唯一遗漏项） |
| request_header tools 唯一性 | ✅ S2c 校验器 + request_header_tools_dup |
| inject_budget budget≥1 | ✅ S2c 校验器 + inject_budget_zero_budget |

dep_graph_events 盘点行 ④⑤（命令/检索不带 dep_graph、事实可选）由
「missing-continue + kind→tool 检查」承载（Python docstring D3 边界同构），
Rust 逐行镜像、无语义遗漏——登记为此前审计表格表述省略。

## 3. 翻转前裁决清单（S2d 输入，更新）

1. `console_order_written` / `console_order_rejected`：写单面退役后
   rejected-without-written 的目标语义（放宽 conformance vs 生产收口）。
2. **probe_accuracy 封存工具翻转语义**（本次补登）：R1 封存工具（如
   run_tests）探针翻转不进声明面，可产出双侧一致的违规 journal——registry
   翻转/S3 门禁改接前需裁决（放宽规则 or 生产收口翻转↔header 不变量）。

## 4. 复审后验证

| 验证 | 结果 |
|---|---|
| spec 表（230 场景 × 31 族 = **7130 裁决格**，场景数精确钉死） | ✅ |
| Rust↔Python 对拍（246 语料 × 31 = **7626 格 0 差**） | ✅ |
| orz-assurance 全量（lib 204 + 集成 9/9/1，含新增 S2c e2e 负测） | ✅ 0 failed |
| `cargo fmt --check` / clippy（仅 lif/mod.rs 存量 1 项不变） | ✅ 净 |
| `cargo check --workspace` | ✅ 零警告 |
| manifest 重算（1438 条，orz `a7981654` 内容） | ✅ |
| 仓库门禁 `check_repository.py` | ✅ Exit 0 |
