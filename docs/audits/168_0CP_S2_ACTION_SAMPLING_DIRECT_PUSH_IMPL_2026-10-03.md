# 168 批：0cp S2 落码——RLI 动作触发式采样与提醒附注式直投（2026-10-03）

> **日期**：2026-10-03；**用户令**：「请进行0cp S2吧」（本会话放行；设计权威＝[`RLI_ACTION_SAMPLING_NOTICE_PUSH_DESIGN`](../RLI_ACTION_SAMPLING_NOTICE_PUSH_DESIGN.md) v1.2 定稿〔163/166/167 批，D1–D7 七决策全定案、D7 用户确认「我同意你做的设计取舍，先这么确定」〕）。
> **形态**：主会话直接执行（子仓落码＋契约面＋ADR 转录＋测试钉）。
> **计数**：不变 **59**（0cp S1→S2 推进，闭合随 S4）。
> **子仓提交**：orz `（本批提交）`；父仓台账随批。

---

## §1 落码范围（设计稿 §6 S2 口径全兑现）

### §1.1 rli.rs（orz-assurance，采样层与提醒层）

- **D1 采样层动作化＋网格退役**：`fill_grid_gaps` 整体删除，`RLI_GRID_SECS`／`RLI_GRID_FILL_MAX` 常数与 `grid_samples` 计数面（字段／访问器／快照字段／PULL 头「网格补点 M」文案）随之退役；`note_sample_point` 只由决策轮＋工具完成事件驱动（动作天然携带特征）；`last_sample_t` 语义转为**动作采样锚**（看门狗 ① 窗起点）。
- **D2 卡死看门狗**：`RLI_WATCHDOG_SAMPLE_SECS=181.0`（TER 180s＋1s 避让注释钉）；`watchdog_due`／`on_watchdog_tick` 三同时判定（武装 ∧ 自动作样 Δt≥181 ∧ 心跳 idle≥181；窗锚缺失＝保守不触发）→ 恰一个时间采样（推进→θ 评估→streak/锚点采样→触发沿走 D4）后休眠（`watchdog_armed` 置 false，动作样重武装）；看门狗样**不**盖动作锚、**不**计 steps（过阈率分母口径不变）。
- **D3**：`RLI_STREAK_K` 5→**3**（注释钉裁决出处）。
- **D4 队列语义改造**：`take_pending_for_push`（取走即置位 `delivered`＝附注已装配，返回已装配态克隆）；队列保留事件历史（cap 16）；`notice_delivery_stats` 保留（投递率＝附注装配数／fire 数；受阻/余量两计数面随 pull 投递退役冻结）。
- **D6 注解格式**：`RLI_T_HAT_ANNOTATION`（「T̂(单轮工具周期会话自适应估计，秒)」单一源）＋`t_hat_trend_text`（前值→现值↑/↓、无成因；<0.05s 不虚构趋势）——四类提醒行全部自带；`t_hat_prev` 随快照持久（跨 prompt 趋势续接）。
- **D7 域事件提醒二合一**：新 `RliNoticeKind::DomainSpikeEntry`——自判域切换当刻即提醒（无需等稳定）；回归端（Stuck/LowProgress→Normal）与 bootstrap（Start→首域）不提醒只计数（`spike_entries`/`spike_returns` 随快照持久）；统一行格式「域事件: …（进N回M）；T̂注解＋趋势；err/slow/stall 现值；近N: 成x误y拒z〔仅读数非阻断〕」；`recent_outcomes` 近 5 动作窗（live-only）；域机器零改动（spike 判定/状态机归 0am）；切换轮与稳定确认轮互斥（确认要求连续 ≥3 轮无切换）；每轮至多一条域类提醒让位规则扩展至 spike 提醒。
- **预算钉**：四类行最坏形态本体＋注解 ≤240B（`notice_texts_stay_within_budget` 扩到四类＋T̂ 注解存在性断言）；CoverageGap 行压缩改版（「驻留N/中位M；域失配N轮」）与注解收档以守住 240B。

### §1.2 controller.rs（orz-loop，装配/渲染面）

- `lif` 字段 `Mutex` → `Arc<Mutex<LifEngine>>`（读法经 Deref 不变）；新增 `lif_shared`（看门狗任务共享）与 `take_pending_rli_notices`（D4 直投取走）。
- **pull-delta 头 RLI 提醒段整体退役**（0bh ① 装配块移除）：投递改直投后头段＝水位标＋分区增量徽章（256 B 帽不变、徽章折叠纪律不变）——162 批实锤 16/16 未投递的面随之关闭；头段投递不走 journal 的盲区随之消除。
- `rli.now` 面头同步（「采样 N」无网格补点）；近提醒行随 D6/D7 变长，超预算让位折断纪律照旧（明细仍可 `selector=channels` 折叠读取）。

### §1.3 agent_loop.rs（D4 直投装配点＋D2 运行时任务）

- **D4**：主车道每轮末（lif_domain 连带记录同块）——存在 Tool 消息时 `take_pending_rli_notices`，行文以「\nRLI提醒: …」附于回传内容后（范式同 0ar 倒数行），并逐条落 `mechanical_audit_update{kind:"rli_notice", payload:{key:"rli.notice.<kind>", round, summary, anomaly:null}}` journal 行（盲区闭环；不进报告块）。触发后无后续工具批＝不跨会话补投（队列留痕）。
- **D2 运行时任务**：`spawn_rli_watchdog`——主车道每 run 一个 5s 级 interval 任务（select! 于取消令牌与 tick；`RliWatchdogGuard` Drop 取消＝触发条件③「run 仍活跃」由任务生命周期保证）；判定与采样在 lif 锁内短临界区、不跨 await 持锁；心跳 `ActivityClock.idle()` 为「无模型流式输出活动」量尺（SSE 帧/chunk/journal 皆盖章）；None 心跳与非主车道不挂。

### §1.4 契约面（schema＋Python 冻结镜像＋钉子）

- runtime schema `mechanical-audit-update-event-payload-v0.2`：kind 枚举 +`rli_notice`（第 10 值）、uniform 分支 if-enum 同步、描述补 rli_notice 条目与键形 `rli.notice.<kind>`；Python 冻结镜像（`run_event_journal_validation.py`）kind 集合与键形清单同步。
- `mechanical_audit.rs`：`KIND_RLI_NOTICE` 常量＋`rli_notice_key`（四触发源 snake_case 单一源）＋schema 互证钉子两枚举数组同步；模块头对象键清单补行。
- **ADR-0010 转录**：§14.83（v1.84）七项落卷（D5＝对 152 批 §5.2 的局部取代声明；§14.81 关系＝LIF 提醒面行为修订、非新增 PULL 面）＋主文件冻结版本补记。

## §2 测试钉（设计稿 §5 判据 → 测试映射）

| 判据 | 钉 |
|---|---|
| 1 streak 只由动作样累计 | `action_samples_only_and_watchdog_single_shot`（100s 空档零补点；原网格批同点产 9 补点样） |
| 2 看门狗恰一样后休眠 | 同上（三同时/休眠/重武装/有活动不触发/窗锚缺失保守/steps 不变六断言）＋运行时任务随 run 存活（spawn 守卫） |
| 3 k=3 | `streak_crossing_pushes_single_notice_and_marks_delivered`（err×3 恰一提醒；沿用例按 k=3） |
| 4 直投＋同刻 journal | `rli_notice_direct_push_attaches_line_and_journals`（agent_loop 级：3 轮超时命令→第 3 轮回传附注行〔末请求恰一份〕＋journal `kind=rli_notice, key=rli.notice.streak_crossed` 恰一条＋delivered=1＋二次取走空） |
| 5 五通道逐位对拍 | `replay_is_bit_exact`／`snapshot_roundtrip_continuation_stays_close`（既有钉全绿＝动力学未被采样层改动波及） |
| 6 D6 注解格式 | `notice_texts_stay_within_budget`（四类行 T̂ 注解存在性＋240B）＋streak/coverage 测试行内断言 |
| 7 D7 二合一 | `domain_spike_entry_notifies_immediately_return_is_silent`（进入当刻提醒〔行格式四要素〕／回归静默计数／bootstrap 静默／稳定确认触发源不变） |

## §3 验证读数（Windows 本机，PROTOC 显式＝仓内 `orz/bin/protoc.exe`）

- `cargo test -p orz-assurance --lib`＝**279/0**；`cargo test -p orz-loop --lib`＝**847/0**（3 ignored）。
- `cargo test -p orz-bin`＝**60/0**（16+12ig／2／15／24／2／1 逐靶同 146 账面）；`cargo test -p orz-tools --lib`＝2990/0；Python `MechanicalAudit` 校验 3/0。
- clippy（orz-assurance＋orz-loop，--all-targets）差分 **107 → 106（净 −1，零新增）**；fmt 触碰五文件零新增 diff（未触碰面预存漂移〔acaf/compact/console 等〕照 148 批口径另记不动）。
- 环境面两件（预存实证，非本批引入）：①`user_cancel_closes_pending_activations_before_run_cancelled` 竞态用例 **stash 差分实证干净树同败**（148 批已登记的 30ms 取消时序负载敏感）；②orz-host 超时/截断类 5–6 例环境敏感失败（干净树 6 败、本批树 5 败＝未触碰 crate）。D 盘页面文件不足（os error 1455）一次——清 incremental＋清损坏 rlib 后恢复（148 批先例形态）。
- 验证边界：S3 进体（双平台载体重建）未跑——沿用 164 批「暂时不重建」口径，构建进体随 0cp S3／0cn S3 窗口；本批验证为 dev profile 源态验证。

## §4 台账

- TODO：`P1-0cp` S2 勾选闭合＋头部指针行/计数行（计数不变 59）。
- BACKLOG：本批记录指针＋`0cp` 节 S2 闭合行＋P1 总览行同步。
- BACKLOG 第二卷：§1.121（本批流水）。
- 索引：`AUTH-RLI-ACTION-SAMPLING-NOTICE-PUSH` 条目 S2 达成＋头行 v4.137 → **v4.138**＋`pending` 桶 RLI 条目注 S2。
- 机械门禁：落账后复跑 `check_repository.py`（见 §6 补记）。

## §5 关键词

168 批、0cp S2、动作触发式采样、网格退役、看门狗 181s、k=5→3、附注式直投、
rli_notice kind、rli.notice.<kind>、盲区闭环、pull-delta 头投递退役、
spike 即提醒、域事件提醒二合一、T̂ 注解＋趋势、240B、ADR §14.83（v1.84）、
局部取代 152 §5.2、计数不变 59。

---

## §6 补记（落账后门禁）

- 机械门禁：落账后复跑 `check_repository.py`＝error_count **0**、`valid: true`（2026-10-03 实测）；父仓 orz 源清单随批再生（`generate_orz_source_manifest.py`，1,485 条）；三条台账头部行随收口收缩至 1,200 字符门内。
