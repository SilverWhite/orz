# 任务 D S2d 批 1 实施审计：写单链判官规则退役（2026-09-06）

> **性质**：任务 D S2d 翻转前裁决清单**批 1（裁决一）实施审计**——用户
> 2026-09-06 放行后落地。裁决登记：
> [`TASK_D_S2D_FLIP_ADJUDICATION_2026-09-06`](TASK_D_S2D_FLIP_ADJUDICATION_2026-09-06.md)；
> 设计转录：ADR-0010 §14.57。orz 提交：`362b6071`（feat/fusion-architecture）。

## 1. 实施内容

### 1.1 ADR-0010 §14.57 转录（裁决一设计权威化）

- 新增 `### 14.57 v1.57 补写裁决索引（2026-09-06）`：写单链判官规则退役
  裁决四项（written 整体退役且不转负检 / rejected 收窄为形状不变量 /
  先读后写硬门不受影响 / 写单复活重新立项边界）。
- 头部冻结版本行补记 v1.56（`.gsa` 会话卷，原批漏记）并追加 v1.57。

### 1.2 Python 法官同步（`assurance/run_event_journal_validation.py`）

- **`_verify_v02_console_order_written` 整体删除**（原 740-818 行：机械戳
  + order id 唯一 + 在先 `blackboard_action_write` 背书核对），并从
  `validate_journal_text` 调用区摘除（调用点留退役注记）。
- **`_verify_v02_console_order_rejected` 收窄**：删除摩擦子规则两处
  （在先同 run written 前置 + round/plan_epoch/run_id 戳一致），保留形状
  子规则三处（phase/step/code 三元组封闭、reason 非空、每 order 至多一
  条）；docstring 登记收窄裁决与不转负检边界。
- payload registry 中 `console_order_written` schema 条目保留（schema 面
  不动，历史事件仍按 payload schema 校验），注释标注规则退役。
- `console_order_rejected` schema 条目不动。

### 1.3 Rust conformance 同步（orz `362b6071`）

- `journal/families_s2c.rs`：`verify_console_order_written` 整体删除；
  `verify_console_order_rejected` 同步收窄（删 written_by_run 前置查找 +
  戳一致核对，保留三元组封闭 / reason / 每 order 一条）；`verify_s2c_family`
  派发摘除 written 臂（留退役注记）。
- `journal/families.rs`：`S2C_FAMILIES` / `ALL_FAMILIES` 摘除
  `console_order_written`（**31 → 30 族**），文档注释登记退役裁决；
  对拍脚本 Python 函数映射同步摘除；场景计数守卫 230 → 229。
- `journal/conformance.rs` 经 `verify_all_families` 自动随 30 族生效，
  零改动。

### 1.4 对拍/表驱动测试改退役口径（`families.rs` 测试区）

- **written 三场景转历史回放合法守卫**（全族零报错 = 实证「不转负检」）：
  `console_order_written_ok`（有背书）/ `console_order_written_duplicate`
  （重复 order_id 不再报错）/ `console_order_written_unbacked`（无在先
  write 不再报错）。
- 删除已退役形状规则的负例场景：`console_order_written_write_reuse` /
  `cow_action_empty` / `cow_step_id_empty`。
- **rejected 场景改退役口径**：`console_order_rejected_ok` 改为无 written
  纯拒绝正例（证明前置摩擦退役）；新增
  `console_order_rejected_stamp_drift_legal`（written 在场且戳漂移仍合法，
  实证退役摩擦不 covert 生效）；删除 `console_order_rejected_unwritten`
  （与 ok 合并）/ `console_order_rejected_stamp_mismatch`（摩擦已退役）。
- 新增形状不变量负例三场景：`console_order_rejected_duplicate`（重复
  order_id）/ `console_order_rejected_empty_reason`（reason 空串）/
  `console_order_rejected_issue_bad_step`（issue 步 execute 越界）；保留
  `bad_phase` / `pre_issue_bad_code` 并剥离 written 前置事件。
- 期望表：written 族期望块整块移除；rejected 期望块更新为五负例。

## 2. 验收

| 项 | 结果 |
|---|---|
| spec 表驱动 `family_verdicts_match_spec_table`：**229 场景 × 30 族 = 6870 裁决格**（written 三历史守卫全族零报错 + rejected 五负例逐格核验） | ✅ 1 passed |
| **Rust↔Python 逐族对拍** `s2b_family_verdicts_match_python`：同一语料双侧 0 差——**247 语料（229 合成 + 12 v0.2 + 6 v0.1 fixture）× 30 族 = 7410 格**，精确核算 assert_eq 通过 | ✅ 1 passed |
| Python 判官对全部 18 个历史 fixture 期刊校验（含 2026-08-16~24 时代合法 written 链期刊）：**0 错误**（历史回放合法性实证） | ✅ |
| orz-assurance 全量：**204 lib + 9 + 9 + 1 全绿**；journal 模块 18 测试全绿 | ✅ |
| `cargo check --workspace` 零警告零错误；`cargo fmt --check` 净 | ✅ |
| clippy：仅存量 `lif/mod.rs` Default 可派生告警（与本次无关） | ✅ |

注：上一批登记的「246 语料」口径与本次实测 247（229+18）差 1，系上批
计数登记误差；本次以测试内精确核算断言（`checked == 30 × corpus.len()`）
为权威。

## 3. 裁决影响面对账（对裁决登记 §4 逐项核对）

| 项 | 裁决要求 | 实施结果 |
|---|---|---|
| console_order_written 规则 | Python 退役 + Rust 退役 + 生产零发射不变 + schema 不动 | ✅ 两侧函数与调用/派发删除；生产未触碰；schema 条目保留 |
| console_order_rejected 摩擦子规则 | 两侧退役 | ✅ 前置 + 戳一致两子规则两侧删除，负例转正例守卫 |
| console_order_rejected 形状子规则 | 两侧保留 | ✅ 三元组封闭 / reason 非空 / 每 order 一条保留，三个新负例锁死 |
| probe_accuracy | 判官零改动（批 2 范围） | ✅ 未触碰 |
| 先读后写（read-anchor） | 生产硬门保留 | ✅ 未触碰 |
| 写单复活边界 | 重新立项登记 | ✅ §14.57 第 4 项转录 |

## 4. 遗留与下一步

- 批 2（裁决二）：`tool_probe` 记账口径收窄（生产侧唯一代码改动），判官
  两侧零改动——待用户放行。
- 两批完成后 S2d 翻转前裁决清单清空 → S2d（31→30 族全量对拍矩阵落盘 +
  registry 翻转准备）→ S3（门禁改接 Rust 法官）→ S4（Python 双法官退役
  收口）。族清单常量已按 30 族更新，S2d 矩阵口径随批 2 后落盘。
