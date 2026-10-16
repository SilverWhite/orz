# 278 批：0de S2 落码＋钉子——治理摩擦综合四件进体（orz `644f6b01`）（2026-10-16）

> **用户令**：「请进行S2吧」（277 批 S1 设计 D1–D7 定案后的落码放行）。
> **性质**：**落码批**——orz 子仓四文件（`orz-loop/src/agent_loop.rs`＋
> `orz-loop/src/context_scale.rs`＋`orz-web/assets/app/projection.js`＋
> `orz-web/tests/frontend_smoke.mjs`，582+/88−）＋父仓 dated 档转录随批；零跑批；
> **计数不变 61**。
> **结论先行**：D1 H1 开窗前置＝可压集合非空＋D3 迟交摘要收束同消息豁免（事发
> 主修）＋D5 区间行括号注解段剥除＋D6 投影双字段兼容＋D7 `rounds_since` 计数器
> 贯通，全部落码；钉 N1–N5 全绿；**loop 877/0**（+4 钉；含四个既有窗口钉按 D1
> 语义迁移）＋**web 47/0**＋前端冒烟全过＋host lib 单线程 **365/0**＋bin check 净
> ＋clippy 96=96 零新增＋fmt 触碰区净。

## §1 落码面（orz `644f6b01`，四文件）

| 件 | 落点 | 内容 |
|---|---|---|
| D1 | `agent_loop.rs` 阶梯块 | H1 臂：`t1_compressible == 0` ⇒ 只注入 `hard_reminder_block` 本体（信息面），不附窗口块；`open_window(HardReminder) = t1_compressible > 0`（单源复用 T1 既有判定，零新增扫描）；审计 `form=reminder_only_no_compressible`（水位照记） |
| D3 | `agent_loop.rs` | `pending_semantic` 二元组 → 三元组（+`stop_declared`）——识别点（`extract_model_stop`）一次性判定随摘要固定；loop-top/run 尾两消费点传入 `compress_blocks_now`；三条告知路径（区间未识别/未落地/台账写失败）按旗标**只落审计不注入**；审计与 journal 可观测性零变化 |
| D5 | `context_scale.rs` | `strip_paren_annotations`：分隔符切分**前**剥除平衡括号注解段（全角/半角按字符扫描配平；未闭合剥至段尾）；`Unrecognized` 仍携带原文行；模块注释言明「区间写在括号外」契约口径 |
| D6 | `projection.js` | `context_compressed` 模板双字段兼容：`estimated_tokens_after` 缺位回退 `target_tokens`（clear 事件「估 ~0K」根修；model_summary 两字段恒等值取值不变） |
| D7 | `agent_loop.rs` | `compress_blocks_now` 增 `rounds_since: u32` 参数；事件 `rounds_since_last_compaction` 写真值（替换硬编码 0）；loop-top/run 尾两消费点贯通既有 `rounds_since_compact`；老写点（机械模板轨）不动 |

## §2 钉子（N1–N5 全绿；0bk/0bm/0bs ⑥/0bn 既有钉不翻）

- **N1** `incident_annotation_with_fullwidth_comma_parses_to_its_outside_range`：
  事发行原样（journal seq 556 逐字）⇒ `Specified([33])`；半角括号注解内逗号不切
  段；未闭合括号剥至段尾；`1-4, abc`／`全部`／纯注解行仍 `Unrecognized`。
- **N2** `h1_window_is_suppressed_when_the_compressible_set_is_empty`：空集合越
  H1 ⇒ 硬提醒本体在、窗口块与剩余轮提示全无、审计 `form=reminder_only_no_compressible`、
  无 `model_compression` 收口事件；对照臂＝四个迁移后既有窗口钉（有块照常开窗）。
- **N3** `stop_declared_summary_lands_audit_without_injecting_notice`：摘要＋
  `[RUN_END] conclude` 同消息 ⇒ 未落地审计照落、无「压缩回执」注入；对照臂＝
  无 STOP 声明 ⇒ 照常注入（0bs ⑥ 语义保留）。
- **N4** `block_compaction_event_carries_the_true_rounds_since_counter`：四模型轮
  （两工具＋门轮＋终轮）后 run 尾消费 ⇒ `rounds_since_last_compaction == 4`（≥1 断言
  ＋精确值钉）。
- **N5** `frontend_smoke.mjs` §10：clear 形 payload（无 `estimated_tokens_after`、
  `target_tokens=171261`）⇒ 渲染「估 ~171K」非「估 ~0K」；model_summary 形取值不变。

## §3 实施勘误与迁移（如实记）

1. **四个既有窗口钉迁移**（`compression_window_participates_via_blackboard_write…`
   ／`…dispatches_non_write_calls…`／`…text_only_rounds…`／`…close_survives_budget_
   exhaustion_break`）：原配置 slider=100M ⇒ 零可压块，D1 语义下窗口不再开启。
   迁移＝**小结果轮在前、肥胖结果轮在后**两轮成块（`SeqOutputsHost` 按序吐结果）
   ＋ H1 抬 1K→2K。机理＝**滑块按整轮粒度切**（`slider_start`＝最近 slider 估算
   的连续完整轮起点）：单个肥胖轮把滑块起点锚在自己身上 ⇒ 零分块、H1 首火即被
   `reminder_only_no_compressible` 抑制并消费闩位（首版迁移踩此坑，残留 journal
   实证后定形）。
2. **N3/N4 首版驱动缺门实质**：反例门有 `answer_gate_has_substance` 收窄（0bi ⑩）
   ——零工具轮 run 门不触发 ⇒ 终轮直接终局、摘要走 run 尾。补前置工具轮后与事发
   「loop-top 消费＋逼轮」形态同形。
3. **预存 flaky 登记**：`retrieval::dispatch::user_cancel_closes_pending_activations_
   before_run_cancelled` 单跑一次失败（干净 HEAD 同样失败）＝负载敏感预存，全量
   复跑即过（877/0）——与严格审查「负载敏感测试」已知形态一致，未收编。

## §4 验证读数

- `cargo test -p orz-loop`：**877 passed / 0 failed**（3 ignored；+4 钉）。
- `cargo test -p orz-web`：**47/0**（基线持平）；`node tests/frontend_smoke.mjs`
  全过（+N5 节）。
- `cargo test -p orz-host --lib -- --test-threads=1`：**365/0**（276 基线持平）。
- `cargo check -p orz-bin`：净。
- clippy（`-p orz-loop --all-targets`）：**96 == 96**（stash 前后对比零新增；触碰
  文件内两处告警均为预存测试代码）。
- fmt：`cargo fmt -p orz-loop -- --check` 净（N1 事发行一行超宽随批修正）。

## §5 边界

1. 零跑批、零重建（S3 载体重建进体待放行；字节判据随批可行＝
   `form=reminder_only_no_compressible` 与 `rounds_since_last_compaction` 事件字面）；
   可与 0dd S3 合并同一次重建（0cz/0da 先例）。
2. orz 推送另行放行；父仓提交含 orz 指针 bump（`4749f6b1` → `644f6b01`）。
3. 计数不变 61（S2 非闭合批）。
4. `user_cancel` 预存 flaky 如实登记、未收编（§3.3）。

## §6 关键词

278 批、0de S2、治理摩擦综合落码、H1 开窗可压集合前置、reminder_only_no_compressible、
收束同消息豁免、stop_declared 旗标、括号注解段剥除、rounds_since 贯通、投影双字段
兼容、N1–N5、滑块整轮粒度、SeqOutputsHost 迁移、loop 877/0、web 47/0、host 365/0、
clippy 96=96、orz 644f6b01、计数 61 不变、索引 v4.251。
