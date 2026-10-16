# 上下文治理摩擦综合设计——收束后迟交摘要逼轮＋区间行解析＋投影字段＋遥测贯通（0de S1）

> **批次**：277（0de S1 勘定；零源码、零 API、零跑批；本批不重建载体）。**日期**：2026-10-16。
> **状态**：S1 定稿，D1–D7 自裁在案（267 批档 §5 批序授权）；S2 落码待放行。
> **证据基线**：事发 run journal `RUN-2b1f85d2-1`（工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹\`，
> 载体 0.9.0）逐事件只读复核查证；HEAD＝orz `4749f6b1`。

## §0 事实与定位（为什么做）

267 批立项时四件（F1–F4）的根因描述基于追查报告转述；本批对事发 journal 做了 seq 级
只读重建，**两处机理描述需勘误**（§1），修复落点随之更精确。事发尾段（全部本地时刻
＝UTC+8）：

| seq | 时刻 | 事件 | 要点 |
|---|---|---|---|
| 523 | 19:00:04 | `mechanical_audit_update` `context_scale:320k` | H1 硬提醒触发（面估算 337,261；`blocks=33`）⇒ 开压缩窗口 |
| 525–535 | 19:02:26–27 | model_output→plan_write→`context_compressed(clear)` | 模型在窗内主动清零（96 轮，364,589→148,186；handover 先写黑板） |
| 536 | 19:02:27 | `mechanical_audit_update` `model_compression` | **窗口按设计收口：`model_participated=true rounds_used=1`**（handover 写数 19→21 翻参与位） |
| 549–554 | 19:03:45 | model_output＋counterexample_gate＋orientation_checkpoint | 终报前收尾门 |
| 555 | 19:05:19 | model_output | 模型终轮消息＝**自愿迟交摘要块＋终报＋`[RUN_END] intent: conclude`**（同消息） |
| 556 | 19:05:19 | `context_scale:block_selection_unrecognized` | loop-top 消费迟交摘要：区间行解析失败（`compressible_blocks=0`；原行见 §3）⇒ 注入「区间未识别」告知块 |
| 558 | 19:05:45 | model_output | 模型被迫多一轮确认（「收到机械面通知：…」） |
| 559 | 19:05:45 | `run_finished{completed}` | run 结束 |

四件实证读数（本 run 全部 7 个 `context_compressed` 事件核对）：

- **F1**：5 个 `model_summary` 事件的窗口面审计＋窗口收口审计全链在案；**事发尾段并无
  「治理后窗口重开」**——H1 窗开启时 33 块有据、收口按设计（seq 536）。真实摩擦＝窗口
  收口后**自愿迟交摘要的 loop-top 消费路径**：可压集合空＋F2 解析失败 ⇒ 注入告知块 ⇒
  强迫多一轮确认（seq 556→558）。另有前段 T1 强制窗未落地升级链（seq 424→434/435，
  18:43–18:47）为同族旁证。
- **F2**：seq 556 实锤原行 `压缩块: 33（窗口期唯一原文块；连同 1–34 区段已由主动清零
  覆盖，本摘要为迟交落账）` 被判 `Unrecognized`——**0bk S1 宽容化其实在役**（267 批
  「解析只认裸 N-M 形」不准确）；真实根因见 §3。
- **F3**：2 个 `clear` 事件（seq 441/534）payload **均无 `estimated_tokens_after` 字段**
  （仅 `target_tokens`）；`projection.js` 模板固定读前者 ⇒ `num()` 取 0 渲「估 ~0K」。
- **F4**：5 个 `model_summary` 事件 `rounds_since_last_compaction` **全部为 0**，与
  「v8 写点硬编码 0」完全一致；老写点（机械模板轨）经参数收真值。

## §1 勘误（267 批 F1/F2 机理描述两处，随批就地勘注）

1. **F1 勘误**：267 批「机械 `context_scale_window` 又开窗一轮：模型被迫进压缩参与轮」
   ——journal 实证该轮**不是窗口**（无 19:00 后的新阶梯 fire、无新窗口块）；H1 窗已于
   seq 536 按设计收口。事发逼轮来自**摘要消费路径的告知注入**（`compress_blocks_now`
   未识别/未落地回执），不是压缩窗口。修复方向由「窗口重武装延迟／收束段豁免」更正为
   「**消费路径收束豁免（D3）＋开窗前置（D1）**」。
2. **F2 勘误**：267 批「解析只认裸 `N-M` 形，只要带全角括号行内说明即解析失败」——
   0bk S1（2026-09-24）宽容化（数字核心＋注解忽略＋全角冒号）已在事发载体 0.9.0 在役，
   其钉用例 `压缩块: 1-62（全部已闭合块；工作现场保留）` 可解析。真实根因＝**分隔符
   切分先于注解容忍**：`parse_block_selection` 先按 `,／，／、` 切段、每段取数字核心；
   事发行注解内含**全角逗号**（「…已由主动清零覆盖**，**本摘要为迟交落账）」），切出的
   第二段「本摘要为迟交落账）」无数字核心 ⇒ 整条 `Unrecognized`。0bk 钉用例注解内用的
   全角**分号**「；」不在分隔集，故当时未暴露。

## §2 F1 裁决（D1–D4）——治理状态感知的机械落点

### D1：H1 开窗前置＝可压集合非空（采）

- 现状：T1 已有 `t1_compressible == 0 ⇒ t1_cut`（走截断兜底路径；截断路径空集已有
  指针化/NoOp 优雅处理）；**H1 无任何前置**——可压集合空时开窗必然空转（无物可压、
  摘要必不落地）。
- 定案：H1（`LadderTier::HardReminder`）开窗前置＝可压集合非空。判定复用 T1 既有
  单源计算（`agent_loop.rs` 阶梯块内 `t1_markers`/`t1_compressible`，先于档位循环
  已算——零新增扫描）。空集合时：**仍注入 `hard_reminder_block` 本体**（信息面保留：
  当前读数＋T1 预告＋清理权限句均可行动），**不附 `compression_window_block`、
  不开窗**；审计 `form=reminder_only_no_compressible`（水位照记）。防「治理后（块
  全清）面回涨越线 ⇒ 开必然空转的窗」形态。
- 不变量：H1「每次 T1 之前都先有过一次硬提醒」承重件不受影响——fire 即回写水位、
  审计照落、提醒块照注入，退役的只是打断式窗口。

### D2：窗内早收口（不立——既有语义覆盖，登记防膨胀）

考虑过「窗口在程中可压集合清空 ⇒ 提前收口」。勘定结论：**无可达路径**——`mode=clear`
/`clear_all` 的 `handover` 为**机械必填参数**且机械层先写黑板再施边界（`controller.rs`
注册描述「防裸清护栏」）；摘要产出（含未落地）也翻 `produced_summary`。即窗内一切
治理动作必翻 `participated=true` ⇒ 窗口收口（事发 seq 536 即此语义，`rounds_used=1`）。
「空集合被逼窗」在窗内存续面不可达，不立新机制。

### D3：迟交摘要收束同消息豁免（采——事发形态主修）

- 现状：`pending_semantic` 消费点（loop-top 与 run 尾）调用 `compress_blocks_now`；
  未识别/未落地/台账写失败三路**注入告知块**（0bs ⑥，F12 观测性回执）。run 收尾
  段（模型已声明 `[RUN_END]`）注入 ⇒ 强迫多一轮模型确认才 `run_finished`（事发
  seq 556→558，26 秒＋一次模型调用；模型被迫产出「安抚机械面」文本）。
- 定案：`pending_semantic` **识别点**同测 `extract_model_stop(response.text)`（既有
  机械收束解析面，`model_stop.rs`）——源消息携带 STOP 声明（`conclude`／`pause`
  皆同，模型即将离场）⇒ 豁免旗标随摘要进 `pending_semantic` ⇒ 消费点传入
  `compress_blocks_now` ⇒ 三路**只落审计、不注入告知块**。压缩成功的正常路径不受
  影响（事件照发、无注入）。
- 无损论证：告知块属注册注入文本（绝不写回持久化会话）——run 结束后它不存在于
  任何持久面；run 内它的作用（让模型知道握手未完成）在「模型已声明离场」时不可
  行动。审计事件（`block_selection_unrecognized`／`summary_not_landed`）照落，
  journal 可观测性零损失。
- 与 F2 的配套关系：仅修 F2 时，迟交摘要会改判 `Specified([33])` → 可压集合空 ⇒
  「指定区间在可压集合中无命中」**仍注入、仍逼轮**。D3 是事发形态的**主修**；
  F2 是中途格式失败的一般防。

### D4：收束段独立豁免（不立）

「最终汇报已发出 ⇒ 不再开窗/逼轮」作为独立机制不立：收束段无机械判定面（`RUN_END`
是模型自述、非可信门，不能作为前置门依据）；D1 已防空窗、D3 已防收束逼轮；窗有据
可压时收束段压缩仍属设计内收益。登记为考虑过并否决。

## §3 F2 裁决（D5）——括号注解段剥除

- 定案：`parse_block_selection` 在分隔符切分**之前**剥除**平衡括号注解段**——反复
  移除全角 `（…）` 与半角 `(…)` 平衡段（未闭合括号剥至段尾），余文再走既有切分
  （`,／，／、`）＋数字核心解析。剥除后：
  - 事发行 ⇒ `Specified([33])`（注解内 `1–34`、全角逗号随注解段整体忽略）；
  - 0bk/0bm 既有用例全保持（`1-62（全部已闭合块；工作现场保留）`／`1-4 (already
    closed)`／`压缩块：2, 6（已压过的不重复）`／`1-4、6`）；
  - **非法段仍拒**：`1-4, abc`／`全部` ⇒ `Unrecognized`（0bk ②「不静默退化」不变）。
- 契约面：0bk 回执文案「可带行内说明，如 `压缩块: 1-62（全部已闭合块）`」已是括号
  形态——**解析对齐既有契约，文案零改**；`context_scale.rs` 模块注释随批同步。
  注解内出现的疑似区间token（如事发行 `1–34`）属注解、不参与区间指令——契约口径
  「区间写在括号外」随本批在模块注释言明。

## §4 F3 裁决（D6）——投影模板双字段兼容

- 定案：`orz-web/assets/app/projection.js` `context_compressed` 模板（现 :201，267 批
  记 :198 为行号漂移）改**双字段兼容**：`estimated_tokens_after` 缺位（clear 事件）
  回退 `target_tokens`（两字段语义同＝压缩后模型面估算；model_summary 事件两字段
  恒等值，取值不变）。
- 否决「payload 单源」（clear 事件补 `estimated_tokens_after` 字段）：事件 schema 面
  膨胀＋历史 journal 不回填，双兼容为最小修。web 前端单文件、零契约变更。

## §5 F4 裁决（D7）——计数器贯通两写点

- 定案：`compress_blocks_now` 增 `rounds_since: u32` 参数；`context_compressed` 事件
  `rounds_since_last_compaction` 写真值（替换硬编码 0）。两消费点传既有
  `rounds_since_compact`（loop-top 消费点与 run 尾消费点均在计数器作用域内；计数器
  生命周期：定义 `:2292`、每轮自增 `:3513`、分块压缩成功后归零 `:2822`——消费时点
  读数＝「距上次压缩的轮数」，即字段设计语义）。老写点（机械模板轨 `:1123`）不动。
- 语义注记：`mode=clear` 事件（`context_manage` 路径）本就无此字段（journal 实证
  `rounds_since: None`）——不在本件范围。

## §6 实现触点（S2 清单）

| 件 | 文件 | 触点 |
|---|---|---|
| D1 | `orz-loop/src/agent_loop.rs` | 阶梯块 H1 臂（`:2578` 附近）：`open_window` 加 `t1_compressible > 0`；空集块＝`hard_reminder_block` 本体；审计 `form=reminder_only_no_compressible` |
| D3 | `orz-loop/src/agent_loop.rs` | `pending_semantic` 元组加豁免旗标（识别点 `:3463` 附近测 `extract_model_stop`）；两消费点（`:2779`/`:5531`）传入；`compress_blocks_now` 三告知路径按旗标跳过注入 |
| D5 | `orz-loop/src/context_scale.rs` | `parse_block_selection`（`:699`）前置括号剥除；模块注释同步 |
| D6 | `orz-web/assets/app/projection.js` | `context_compressed` 模板（`:201`）`estimated_tokens_after` 回退 `target_tokens` |
| D7 | `orz-loop/src/agent_loop.rs` | `compress_blocks_now` 签名＋`:1625` 事件字段；两消费点传 `rounds_since_compact` |

## §7 判据与钉子（S2 定案）

- **N1（D5）事发行原样钉**：`压缩块: 33（窗口期唯一原文块；连同 1–34 区段已由主动
  清零覆盖，本摘要为迟交落账）` ⇒ `Specified([33])`；0bk/0bm 既有用例全绿不翻；
  `1-4, abc`／`全部` 仍 `Unrecognized`；新增注解含全角逗号＋en-dash 用例。
- **N2（D1）H1 空集不开窗钉**：极小刻度 ladder 驱动（既有
  `with_context_scale_ladder` 测试道）——可压集合空＋越 H1 线 ⇒ 断言无
  `ModelCompression` 窗口、注入块含硬提醒文本但不含窗口块文本、审计
  `form=reminder_only_no_compressible`；对照臂＝有可压块时照常开窗。
- **N3（D3）收束同消息豁免钉**：带 `[RUN_END] intent: conclude` 的回复产出摘要
  （区间不可落地）⇒ 断言审计事件落、`messages` 尾无告知块；对照臂＝无 STOP 声明
  ⇒ 照常注入（0bs ⑥ 语义不翻）。
- **N4（D7）计数器贯通钉**：N 轮后驱动分块压缩成功 ⇒ 断言事件
  `rounds_since_last_compaction == N`（≥1）；0bn 既有钉不翻。
- **N5（D6）投影对拍钉**：clear 形 payload（无 `estimated_tokens_after`、
  `target_tokens=X`）⇒ 渲染含 X、不含「估 ~0K」（web 测试面）。

## §8 批序

S1 勘定（本批）→ **S2 落码＋钉子**（待放行；D1/D3/D5/D7 落 `orz-loop`＋D6 落
`orz-web`，读数＝host lib 单线程全绿＋web 全绿＋clippy 触碰面零新增）→ S3 载体
重建进体（字节判据随 S2：`form=reminder_only_no_compressible` 与
`rounds_since_last_compaction` 事件字面可作字面判据）→ S4 真机复验（长文写作轮
收束段无逼轮＋带全角括号注解摘要可解析＋UI 估数非 0）。

## §9 边界

1. 本批零源码、零跑批、零重建；orz 子仓不动（HEAD `4749f6b1`）。
2. 693K 自拉回 vs 900K 必压线间距＝用户裁决「已经足够」，不处理（267 §1 维持）。
3. 模型两次主动 clear 与摘要质量＝设计内良性使用（267 §4 维持）；0cz/0da 工具线不动。
4. 267 批档 §3 F1/F2 两条机理描述就地勘注（随批）；索引 canonical 条目随 S1 达成
   更新时同步勘正表述。
5. D2/D4 两项不立裁决＝登记防膨胀，不作开放项、不计数。

## §10 裁决记录（D1–D7 浓缩）

- **D1** H1 开窗前置＝可压集合非空（单源复用 `t1_compressible`）；空集注入提醒本体、
  不开窗、`form=reminder_only_no_compressible`。采。
- **D2** 窗内早收口——不立（clear/clear_all 机械强制 handover 写、摘要产出翻
  participated ⇒ 治理必收口，无空集逼窗可达路径）。
- **D3** 迟交摘要收束同消息豁免——识别点测 `extract_model_stop`，带 STOP 声明 ⇒
  消费点三路只审计不注入。采（事发主修）。
- **D4** 收束段独立豁免——不立（无机械判定面；D1/D3 已覆盖）。
- **D5** F2＝分隔符切分前剥除平衡括号注解段；非法段仍拒；契约文案零改。采。
- **D6** F3＝投影模板双字段兼容（`estimated_tokens_after` 回退 `target_tokens`）；
  否决 payload 加字段。采。
- **D7** F4＝`compress_blocks_now` 增 `rounds_since` 参数写真值；两消费点贯通
  `rounds_since_compact`。采。
