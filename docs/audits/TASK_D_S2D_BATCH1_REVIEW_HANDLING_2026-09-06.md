# 任务 D S2d 批 1 复审处理审计（2026-09-06）

> **性质**：批 1（写单链判官规则退役）实施后三路全面复审的处置登记——
> 语义符合性 + 设计合理性（含生产面核实）/ 实现质量 / 治理登记与数字
> 核验，三路并行只读复审。对象：orz `362b6071` + 父仓库 `ebd918e`。
> 处置提交：orz `141bd2cf`（本批 orz 侧收口）。
> 关联：[批 1 实施审计](TASK_D_S2D_BATCH1_IMPL_AUDIT_2026-09-06.md) /
> [裁决登记](TASK_D_S2D_FLIP_ADJUDICATION_2026-09-06.md) / ADR-0010 §14.57。

## 1. 复审发现与处置总表

| # | 级别 | 发现 | 处置 |
|---|---|---|---|
| R1 | **P1** | 父仓库 Python 测试面未随退役同步：`runtime/tests/test_run_event_journal_validation.py` 两测试 import 已删 `_verify_v02_console_order_written`（收集即 ImportError）+ 一测试断言已退役摩擦行为（无在先 written 报错 / 戳漂移报错），实测 **3 failed / 255 passed**。实施审计验收面（orz 侧 + 历史 fixture 回放）漏掉判官自身单测面，验收声明「全绿」存在盲区 | **采纳修复**：三个测试改写为退役口径——written 两测试转「不转负检」合法守卫（backed/duplicate/unbacked/多订单链/write_call_id 错配与复用全部零报错）、rejected 摩擦测试转退役口径（无 written 拒绝合法 + 戳漂移合法 + 每 order 一条保留 + 跨 run 隔离正例）。修后 **258 passed / 0 failed** |
| R2 | P2 | 「rejected 发射点保留/仍在发射」定性失准：五个发射点代码在（`console_exec.rs:485/529/594/630/759`）但运行时休眠——订单槽唯一生产写入面 `blackboard_action_write` 被 §14.39 调用面窄门无条件拒发（`host_exec.rs:328-331` `retired_tool_denied`，先于休眠 handler `:2127`；快照恢复剥单 `blackboard.rs:707`），rejected 与 written 同源休眠。代码保留意义与裁决语义不受影响，登记文本前提失实 | **登记修正**：ADR-0010 §14.57 项 2 改「发射点代码保留、运行时休眠」；裁决登记文档补复审勘误注；`console_exec.rs` 统一构造器注释同步（orz `141bd2cf`） |
| R3 | P2 | 休眠 read-anchor 拒单落点码表冲突：`console_exec.rs:627-638` 发 `pre_issue/protocol/content_anchor_mismatch`，不在保留的 pre_issue 三码封闭集（order_stale/step_not_done/budget_insufficient）内——退役前既有（旧规则同为封闭 3 码），休眠期对真实期刊零影响（18 历史 fixture 0 错误可证），但写单复活后首个锚点拒单即被判官打错 | **登记修正**：ADR-0010 §14.57 项 4 复活边界补「复活立项须一并裁决该码表豁免或扩集」；裁决登记勘误注同步 |
| R4 | P3 | Rust spec 表覆盖缺口：pre_issue 且 step≠protocol 负例、reason 非字符串（null）负例、跨 run duplicate 隔离正例缺失（Python 测试面前两者已有、跨 run 亦缺） | **采纳**：补 3 场景 `console_order_rejected_pre_issue_bad_step` / `console_order_rejected_reason_null` / `console_order_rejected_cross_run_duplicate_legal`（跨 run 事件手工构造信封——`ev()` 信封 run_id 固定 run-1，首轮实施即踩此坑被 spec 表断言当场拦下）；期望表 +3 格；计数守卫 229→232 |
| R5 | P3 | written 三历史守卫场景「全族零报错」仅靠对拍 parity 间接保证（两侧同错时 parity 假绿） | **采纳**：`family_verdicts_match_spec_table` 增显式断言——三场景 `verify_all_families` 必须为空（防退役漂移成单侧负检） |
| R6 | P3 | 生产注释残留退役前语义：`console_exec.rs:401-402` 统一构造器 doc 仍称与 `console_order_written` 记录「verifier 交叉核对」 | **采纳**：注释同步退役语义 + 休眠登记（orz `141bd2cf`） |
| R7 | P3 | Python docstring 内函数名被换行拆断（`_verify_v02_console_⏎order_written`） | **采纳**：随 R1 测试改写一并消除（docstring 重写，不再引用该断名） |
| R8 | P3 | 实施审计勘误「差 1」量级不精确：上批（4c1f8d1）登记 246 语料时 fixture 已恒为 18（v0.2 轨 54560b4 起恒 12 + v0.1 恒 6）、场景 230，真实语料 248——登记误差实为 **2**；「差 1」混入了本批合法的场景数变化（230→229） | **采纳修正**：批 1 实施审计 §2 注改为精确口径；BACKLOG/TODO S2c 历史行补勘误指针（历史数字不改） |
| R9 | P3 | orz `362b6071` 提交信息「229 合成+246 fixture」笔误（246 误标为 fixture 数；实际 fixture 18、总语料 247） | **登记不改**（提交已落、hash 被引用；正确数字以测试内精确核算断言与父提交 message 为准） |
| R10 | P3 | 旧错误数字「246 语料×31=7626」在三处历史登记（BACKLOG S2c 行 / TODO S2c 行 / S2c 复审处理）留存无勘误指针 | **采纳（指针级）**：BACKLOG/TODO S2c 行补「语料计数勘误见批 1 实施审计」；历史行数字不改 |
| R11 | P3 | schema `console-order-rejected` order_id description 残留「matches the preceding console_order_written」摩擦语义 | **登记不改**（schema 冻结是裁决明文；描述性文字，不影响校验） |
| R12 | P3 | 写单复活无机械绊线（written 发射代码物理保留 + 无负检） | **登记不改**（裁决自主选择的残余风险，§14.57 项 4 已登记流程拦截；R3 码表冲突已并入复活立项清单） |

## 2. 复审确认成立的关键结论（无发现项摘要）

- **生产零发射前提成立**：written 唯一发射点位于窄门后休眠 handler（不可达）；
  全仓无「出现 written 即报错」的隐式负检；fixture 生成器产出含 written 的
  历史期刊属合法回放面。
- **转录保真**：§14.57 与裁决登记五项目标语义逐项有落点，无实质漏项
  （登记项 2/3 合并为 §14.57 项 2，内容完整）。
- **依赖面干净**：判官代码删净、无孤儿 helper（`py_value_eq` 等仍被其他族
  使用）、派发臂/S2C_FAMILIES/ALL_FAMILIES/对拍脚本映射四方一致（30 族）。
- **Python↔Rust 语义对齐**：收窄后 rejected 校验器逐分支一致、消息结构性
  同措辞；`payload["order_id"]` 直接下标安全（payload_errors 先行门控 +
  schema required）。
- **登记四方一致 + 索引豁免成立**：实施审计/裁决登记/BACKLOG/TODO 状态、
  hash、数字互洽；索引 §0.5 纪律下批 1 不更新 CLI_PROJECT_INDEX 正确
  （S2b/S2c 先例一致）。
- **提交规范**：两提交无夹带、message 合惯例（R9 笔误除外）。

## 3. 复审处理批验证（orz `141bd2cf` + 父仓库本提交）

| 项 | 结果 |
|---|---|
| spec 表 `family_verdicts_match_spec_table`：**232 场景 × 30 族 = 6960 裁决格**（+3 场景；written 三守卫显式全族零报错断言） | ✅ 1 passed |
| Rust↔Python 对拍：**250 语料（232 合成 + 18 fixture）× 30 族 = 7500 格 0 差** | ✅ 1 passed |
| 父仓库判官测试 `runtime/tests/test_run_event_journal_validation.py`：**258 passed / 0 failed**（修前 3 failed） | ✅ |
| orz-assurance 全量 204 lib 全绿；`cargo check -p orz-loop` / workspace 零警告；fmt 净 | ✅ |
| manifest 重算 + 门禁 `check_repository.py` Exit 0 / valid=true | ✅ |

## 4. 遗留

- 批 2（裁决二：tool_probe 记账口径收窄）待放行——判官两侧零改动，本批
  R4 补的场景与 R5 断言对批 2 的 probe_accuracy 复验同样生效。
- S2d（全量对拍矩阵落盘）→ S3（门禁改接）→ S4（Python 双法官退役收口）
  依序推进；S2d 矩阵口径按 30 族 / 232 场景落盘。
