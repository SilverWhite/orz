# 287 批：0df 任务级计费机械记录点——`run_usage_metered` 事件族（S1 勘定＋S2 落码）（2026-10-17）

> 性质：**设计勘定＋落码同批**（用户令「请先进行0df吧」；S1＋S2 同批沿 0cl／0ct
> 先例）。零跑批（未发起任何 orz 会话）；orz 子树落码提交 `6e107c77`（**不推送**）；
> **未重建、未发行**——装机位仍 0.9.1，计量行与 286 缺省开同待下一版载体重建生效。
> 计数 62 不变（0df 于 284 批立项已入计数，本批为实施批）。

## §1 由来（用户令）

- 2026-10-16：「我打算给下一个版本的 orz 配个任务级计费，跟着会话一起存档。」
  ＋「请将这一内容记为待办吧，作为新的机械记录点。」（284 批立项，61 → 62）。
- 2026-10-17 追加：「把配置侧的 reasoning effort 档位写进任务行——与观测到的
  reasoning token 分列」（档位恒定 ≠ 推理量恒定：r4 实证全轮一档下推理占
  completion 76%、逐题 28%–90%）。
- 2026-10-17：「请先进行0df吧」——本批执行。

## §2 S1 定案（设计稿 D1–D8；五项待勘定全数闭合）

设计权威：[`PER_TASK_METERING_DESIGN_2026-10-17`](../PER_TASK_METERING_DESIGN_2026-10-17.md)；
ADR-0010 **§14.86（v1.89）** 随批转录。

- **D1 载体改判**：独立 EventType `run_usage_metered`，**否** 284 档预设的
  mechanical_audit kind 扩档——轮级对象键（key/round/summary/anomaly）语义不合
  run 级终态行；schema「主车道专属」约束与子代理折算相抵；审查卡渲染负担
  （285 F3 幽灵行前科）。同改面沿 face_fingerprint 先例（orz `7a6fe91e`）。
- **D2 粒度**：一 run 一行、终态前；主车道与检索子代理同 journal（子代理轮写主
  run journal），一行覆盖全部车道消耗。SCB 题=run 恰等；TB 形态映射归评测侧。
- **D3 payload**：`model`／`model_calls`／`prompt_tokens`／`cache_hit_tokens`／
  `cache_miss_tokens`／`completion_tokens`／`reasoning_tokens`／`effort_tier`／
  `usage_incomplete` 九字段全 required。**含 `prompt_tokens`**（journal
  `model_output` 缺该口径 ⇒ 折算必须在内存 ModelResponse 到达点，不可事后重放）。
- **D4 金额面**：机械记录**不记金额**——in-band 无自足来源（provider 不报 cost
  ＋余额差分并发不安全＋TB 2.1 端点不可用实证＋账单侧无 API）；journal 哈希链
  不可改写 ⇒ 回填结构性不能改历史行；金额归因＝run 行 token × 金额来源在**派生
  台账**配对（283 方法学机械化，工具另行立项）。**折算（rate card）不入机械记录**
  （两栏分列纪律：陈旧价目即冒称实付温床）。
- **D5 档位设计值**：gateway trait 单读（设计值语义）；空响应链降级是 per-call
  局部量不改 config，降级事实由 `request_header_change` 承载；与观测
  `reasoning_tokens` 分列。
- **D6 折算器宿主**：`SharedLoopServices` 新字段（per-run 复位，沿 `for_new_run`
  隔离先例；非 Option——两车道都要折）。
- **D7 写点**：controller 五终态臂 `record_terminal` 前恰一条；非 completed 臂
  保守 `usage_incomplete=true`；grill 随 `journal: None` 自动丢弃。
- **D8 执法**：`run_started` 增 optional grandfather 锚 `usage_metering: "run-v1"`
  （0q `failure_pipeline` 先例同构＝历史刊零回归）＋ `usage_metering` 法官族
  （Rust 执法＋Python 冻结镜像对拍）。

## §3 S2 变更面（落码；orz `6e107c77`，12 文件 +630/−3）

| 面 | 落点 | 内容 |
|---|---|---|
| 事件族 | `orz-assurance/src/journal/event.rs` | `RunUsageMetered` 变体＋payload 契约注释 |
| 法官族 | `orz-assurance/src/journal/families.rs` | `verify_usage_metering`（恰一条＋终态前＋形状/枚举/非负＋零轮非零和）＋ALL_FAMILIES＋corpus 场景 `usage_metering_ok`／`_violations`（钉值 259→**261**）＋对拍表注册 |
| 折算器 | `orz-loop/src/usage_meter.rs`（新） | `RunUsageMeter`＋3 unit 钉（求和/缺项置 incomplete/零轮形态） |
| 宿主 | `agent_loop.rs` `SharedLoopServices` | `usage_meter` 字段＋`ModelResponse` 到达点折算（取消检查点前）＋流中断/取消两 Err 臂 `mark_incomplete` |
| 档位单读 | `gateway/model.rs`＋`transport.rs` | trait `usage_meter_labels()`（default `unknown/unknown`）＋DeepSeekTransport 返回 `config.model_id`＋thinking 档设计值 |
| 写点 | `controller.rs`＋`host_exec/facts.rs` | 字段＋构造器两处＋`run_turn_inner` 起点复位＋标签单读＋`run_started` 锚＋`journal_usage_meter` helper 五终态臂共用＋两处序列钉校准（`run_turn_full_gate_sequence`／`tool_run text_deltas`） |
| 子代理 | `retrieval/dispatch.rs` | 两 svc 构造点透传同一 meter 引用（子代理轮折入同行） |
| TUI | `orz-tui/src/bridge.rs` | `RunUsageMetered` 降级 Unknown（WriteControlReview／FaceFingerprint 同姿态） |
| runtime 合约 | 父仓 `runtime/` | `run-event-v0.2.schema.json` enum＋payload registry v02 条目＋`run-usage-metered-event-payload-v0.2.schema.json` 新档＋`run-started` schema 锚容忍＋fixtures 三件（envelope valid／payload minimal＋constraint） |
| Python 镜像 | `assurance/run_event_journal_validation.py` | `_verify_v02_usage_metering` 同形＋roster 调用注册 |

## §4 验证读数

- `cargo test -p orz-loop --lib` ⇒ **882 过／0 败**（基线 877＋本批 5 钉：usage_meter
  3＋集成 2〔两 run 恰一条＋`model_calls`=2 复位实证＋锚＋族绿；失败臂保守
  incomplete〕）。
- `cargo test -p orz-assurance --lib` ⇒ **301 过／0 败**（含 Rust↔Python parity
  261 场景 × ALL_FAMILIES 全绿——`usage_metering` 族两 judges 逐格恰等）。
- `cargo test -p orz-tui --lib` ⇒ **172 过／9 败**（同一 ACAF 预存失败集合，零新增）。
- `cargo test -p orz-bin --bins` ⇒ 15/0；`cargo test -p orz-web --all-targets` ⇒
  **47/0**；`node crates/orz-web/tests/frontend_smoke.mjs` ⇒ 全部通过。
- `cargo fmt`（触碰 crate）净；clippy 零新增（新码 collapsible_if 收口后触及面
  告警逐零）。
- 门禁 `python scripts/check_repository.py` ⇒ 目标 `valid: true`（manifest 重生成
  1488 entries、随 orz 提交同步）。

## §5 生效路径（真机面）

- 计量行属**载体面行为**：本批只改源码与文档，**未重建**——装机位（0.9.1）不产
  计量行。生效随下一版载体重建（与 286 缺省开同车）。
- 生效后判据（下一真机轮）：每个 run journal 终态前恰一条 `run_usage_metered`；
  `run_started` 带 `usage_metering: "run-v1"` 锚；五项求和与 provider 账面
  （hit/miss/completion）可对；检索子代理轮折入同行（行 `model_calls` ≥ 主车道
  轮数）；`usage_metering` 法官族零违例。
- 单任务成本归因闭环＝计量行（token，机械自带）＋金额来源（余额差分/账单导出）
  派生台账配对——后者另行立项（283 方学名机械化）。

## §6 台账

- 本档；设计稿 [`PER_TASK_METERING_DESIGN_2026-10-17`](../PER_TASK_METERING_DESIGN_2026-10-17.md)；
  ADR-0010 [`§14.86`](../../adr/ADR-0010-vol-14-addenda-index.md)（v1.89）＋BACKLOG
  `0df` 节 S1/S2 勘注＋索引头行 v4.259 → **v4.260**＋BACKLOG 本批指针行/计数行＋
  TODO 计数行/P1 路由行＋[`BACKLOG 第二卷 §1.227`](../BACKLOG_AND_PRIORITIES_2.md)。
- orz 子树落码提交 `6e107c77`（不推送）；`orz_source_manifest.sha256` 重生成
  （12 触碰件随 1488 entries 全量）。
- 零跑批；未重建、未发行；装机位保持 0.9.1；计数 62 不变。

## §7 关键词

287 批、0df S1＋S2、任务级计费、机械记录点、run_usage_metered、事件族、
face_fingerprint 先例、RunUsageMeter、SharedLoopServices 宿主、双车道统一折算、
prompt_tokens 补账、usage_meter_labels、effort_tier 设计值、reasoning_tokens 分列、
金额不入 journal、哈希链不可改写、派生台账回填、grandfather 锚 run-v1、
usage_metering 法官族、corpus 261、grill 丢弃、门禁 valid true、索引 v4.260。
