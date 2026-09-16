# 0af 落码 ＋ 0ah 收口清理批（2026-09-16）

> 用户指示「请直接进行0af和0ah吧」（2026-09-16）——两项同日落码并提交。
> **0af**＝资源门拒绝文案明确化（P1，2026-09-15 立项，深审摩擦 B 注册；
> orz **`b6ed78d9`**，闭合入账 **36 → 35**）。
> **0ah**＝v7→v8 收口清理批（实施回执 §8.4 登记的待清理项；orz
> **`501447c0`**；0ah 状态维持 `partial` 不变——判据读数／S2／尾批仍待各自
> 放行）。本批零契约面改动（schema／verifier／冻结镜像／fixtures 全不动；
> manifest 重算差异恰 5 文件 ×2 行，1457 条不变）；门禁 `valid: true`
> （error_count 0）。索引条目 `GAP-RESOURCE-GATE-DENIAL-COPY`（转
> `implemented`）／`GAP-DYNAMIC-CONTEXT-SLIDER`（收口批登记）。

## 1. 0af：资源门拒绝文案明确化（orz `b6ed78d9`，2 文件 +186/−11）

落地面（定案边界＝不改 fail-closed 判定逻辑与阈值，只改文案与轴标注）：

1. **模型面定案句**（`resource_gate.rs` 四常量）：按**实际耗尽轴**标注——
   只有储存（卷余量）短 ⇒「宿主机储存资源即将耗尽，无法新增派发，请寻找
   其他方案」；只有内存（commit）短 ⇒「宿主机内存资源即将耗尽…」；双轴同短
   ⇒「宿主机内存/储存资源即将耗尽…」（合并标注）。机械读数（短少明细＋
   `readings` 信封）保持英文原样随附。
2. **混排明示为有意选择**（审查补充③）：模块头新登记段——定案句用用户工作
   语言让模型不必解码术语即可行动，读数保持机械可逐字核对的原样；不做建议
   引擎／恢复指引（定案边界）。
3. **Unknown 档变体句**（审查补充①）：读数不可得（fail-closed 拒绝）改
   「主机资源读数不可得，无法确认余量，已按 fail-closed 规则拒绝新增派发，
   请寻找其他方案（unreadable targets: …）」——读数缺席时**不得断言
   「即将耗尽」**（防不实陈述）。
4. **百分比取整修正**（审查补充②）：headroom 百分比改**一位小数、向下取整**
   （整数数学，无浮点）＋字节直读同行——整数截断曾把实值 24.4% 显示成
   「commit headroom 25% < 25% required」的字面自相矛盾（真机 run 实录）；
   向下取整保证显示值 ≤ 真值 ⇒ 拒绝理由里结构性不可能出现「25.0% <」形态。
5. **连带修复（F-BE-12 残留完成）**：`lib.rs` `call_tool_inner` 汇点的拒绝臂
   此前仍**第二次**调 `evaluate_for_volumes`（S2R 只把档位/回收面改用首次
   判定、拒绝臂漏改）——两次探针可跨档位分歧，正是 F-BE-12 登记要消除的
   形态；本批改用唯一一次判定（`gate_decision`）。

钉子 4 条：`refusal_headline_percent_never_self_contradicts`（实值 24.375%
构造，断言显示 24.3% 且不出现「25% <」）／`denial_headline_names_every_short_axis`
（双轴合并标注）／`headroom_percent_floor_never_rounds_up`（24.99% 陷阱值
不进位）／`resource_gate_judges_once_per_call_tool`（计数探针：一次重活调用
恰一次门判定）。既有断言同步：`heavy_refused_*`（轴标注）／
`unavailable_readings_fail_closed_for_heavy_only`（变体句＋不断言耗尽）／
`any_short_write_target_volume_refuses_the_heavy_action`（储存轴）／
`run_tests_is_gated_like_any_heavy_action`（run_tests 同口径路径的新文案）。

**读数**：orz-host 串行 **332/0/5**（基线 328 ＋4 钉）、fmt 干净、clippy 与
HEAD 基线**逐位一致**（本机实测 58/76；账本旧读数 50/12 系不同时点口径，
本批以同命令对 HEAD 复测为基线比对 ⇒ 零新增）、契约面零改动
（`host_resource_denied.reason` 为自由字符串，生成器 fixture 用通用占位文案
不受影响）。

## 2. 0ah 收口清理批：v7 遗留折叠族退役（orz `501447c0`，3 文件 +149/−1825）

实施回执 §8.4 的登记原文：「`action_ledger` 的折叠／推进族在 v8 主车道生产
零调用……本批保留未删（删面涉及检索车道与多处测试，属独立批）」——本批即
该独立批。落地面：

1. **删除（生产零调用取证后）**：`LedgerFoldState`（有状态折叠点）／
   `advance_fold`（推进＝一次驱逐）／`build_request_view`（折叠态请求视图）／
   桥渲染族（`build_bridge`／`truncate_bridge_to_budget`／
   `truncate_tool_reply_tail`／`has_tool_result_read_back_pointer`）／
   `safe_fold_cut`／`is_round_balanced`／`bridge_estimate_budget`／
   `FOLD_TAIL_CHARS_PER_TOKEN`／`build_collapsed_request`（无状态坍缩渲染器，
   v7 视图取代后零调用）。`action_ledger.rs` **3,145 → 1,422 行**。
2. **`run_template_compact` 退役 `fold_state` 传参**：v8 后唯一生产调用方＝
   **检索/grill 车道 session-end**（主车道 loop-top 触发与收尾压缩已随 R-3
   退役）；保留起点一律按无状态 `collapsed_cut(messages, tail)` 重算，
   `frozen_ledger` 恒 `None`，marker 删除后重算口径不变（400 修复纪律保持：
   守卫路径不动 marker）。
3. **`LoopOutcome.fold_state` 字段退役**（loop 恒为未折叠态，状态不再外传）；
   检索 `dispatch.rs` 同步（去 clone 与传参）。

**保留面（逐项取证非死代码）**：`bridge_cut`（**v8 投影层复用**——
`model_face::slider_start` 主滑块起点即它）／`collapsed_cut`（检索车道收尾
保留起点）／`rounds_before`／`round_ranges`／`is_round_complete`／
`rows_for_round`／`build_ledger_block`（按块压缩行装配）／
`append_ledger_rows(_range)`／`external_row_line`／`ledger_file_path`／
`build_pointer_message`（投影层与 summary 复用）／`capture_run_baseline`／
`render_run_context_block`（v8 face D4 机械段）／`pointerize_*`（v8 指针化面）。
**P2-14 v0.3 折叠快照 marker 分支保留**（`fold_ctx` 参数不动——该域属
`AUTH-COMPACTION-FOLD-SNAPSHOT` 的 S3/S4 待续项，不在本批退役范围）。

**测试面**：25 个死测试删除（safe_fold_cut ×2／build_request_view ×4／
unfolded_view／advance 族 ×3／folded_view_prefix／fold_reset／advance_fold ×2/
bridge_estimate／slider_evicts／bridge_view·truncation ×9）；`collapsed_cut`
直测五件改写为直测存留件（`zero_rounds`／`old_rounds_collapse`／
`multi_result_round`／`incomplete_round`／`mid_history_incomplete`／
`deterministic_output`）。**782 = 807 − 25**——逐个对账，零误伤。

**读数**：orz-loop **782/0/3**、orz-host 串行 332/0/5、orz-assurance **229**、
`cargo check` 可及 crate 全绿（orz-bin／orz-tui／orz-loop／orz-host／
orz-assurance；workspace 全量 check 受本机缺 `protoc` 限制止于 orz-tools-api
构建脚本——既有环境限制，与本批无关，本批未触碰 orz-tools）、
`cargo fmt --all -- --check` 干净、clippy 与 HEAD 基线逐位一致、
`runtime/tests` **361/1**（唯一失败＝既有无关红灯
`test_v02_all_51_event_types_covered`）、门禁 `valid: true`（error_count 0；
manifest 1457 条，差异恰 5 文件 ×2 行）。

## 3. 0ah 批序位置（勘误后 DP-6）

⓪ v7 五连批作废 → ① v8 实施（已落码，orz `4952bc8c`）→ **③ v7→v8 收口
（本批，提前于 ② 执行——收口清理与判据读数互不依赖，登记如实）** →
② 判据读数（真机首件，§9 配方，搭下一轮狗粮 run）→ S2 裁决 → 尾批块轴。
0ah 计数不变（仍为开放项），状态维持 `partial`。

## 4. 未核项与边界

- 0af 定案句的真机效果（模型读中文定案句后的行为改善）待下一轮狗粮 run
  的摩擦读数（同 0ah 五项读数一并收取）。
- 0ah 收口批删除面为纯机械退役（生产行为零变化——被删函数 v8 主车道生产
  零调用、检索车道恒走 fallback 路径）；跨 run 时序与恢复面由既有钉子覆盖。
- 本机 clippy 全量计数（58/76）与账本历史读数（50/12）的口径差异已如实
  登记：本批以「同命令对 HEAD 复测」为基线，逐位一致 ⇒ 零新增。
