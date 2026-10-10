# 模型主控上下文 R2 设计稿——0cz 全面审查处置＋清理操作多样化 replace 族

> 版本 v1.0（S1 设计稿，待放行）；日期 2026-10-11；立项权威＝[`239 批档`](audits/239_0DA_MODEL_CONTEXT_CONTROL_R2_2026-10-11.md)
>（用户三段裁决）。本文是 `MODEL-CONTEXT-CONTROL-R2`（0da）的 S1 设计权威草案；放行后按 §9 批序进入 S2 落码。
> 上游权威＝[`0cz 设计稿`](MODEL_CONTEXT_CONTROL_DESIGN_2026-10-11.md)（S1/S2 已达成，本稿处置其在役面审查发现）。

## §0 定位与哲学（用户裁决，转录）

0cz 把上下文的控制权交给了模型；本轮把**清理的粒度选择权**也交给模型——一键清理／清理黑板／
清理上下文**分开给、按需选择**（裁决②），同时修复审查坐实的「清零后兜底失效」（D1）与
「keep_recent_rounds 三方不一致」（D2），审查发现**全部问题无论大小**全量处置（裁决③）。
任务保留＝**零机制既定姿态**（裁决①：类 0cw 重置形态，黑板与台账不清零、有 plan；S4 观察项）。

## §1 canonical ID 与注册定位

- canonical ID：`MODEL-CONTEXT-CONTROL-R2`（BACKLOG `0da`，P1；239 批立项 58 → 59）。
- **零新工具**：三清理操作全部落位于在役两具（`context_manage` mode 扩展＋`blackboard_write`
  op 扩展）；8 工具面冻结例外维持 +3 不变；WORK_TOOLS 三处同批**零改动**（表成员不变）。
- 命名消歧（O1 立场，见 §6.1）：参数面用 **clear 族命名**；用户裁决原词 replace_context／
  replace_blackboard／replace_all 作为**能力速记**转录于档，不上参数面——`replace` 语义已被
  既有 `blackboard_write op=replace_all`（整节重写）占用，重命名破坏零回归。

## §2 用户已裁要点（S1 前提，不重开）

| # | 裁决 | 出处 |
|---|---|---|
| ① | 任务保留零机制（原任务 prompt 出窗＝既定姿态；S4 观察项「任务漂移」） | 239 批 §2-① |
| ② | 清理操作多样化：一键清理／清理黑板／清理上下文分开给、按需选择 | 239 批 §2-② |
| ③ | 审查发现全量处置，无论大小（D1–D7） | 239 批 §2-③ |
| ④ | 护栏不对称立场（239 批 §4）：上下文臂承继防裸清 handover；黑板臂 journal 在案无需同级护栏；三分区一致放行 | 239 批 §4 |

## §3 S1 现场盘点（2026-10-11，源码实证）

### 3.1 O1 载体盘点：三操作落位与契约面

- **journal `section` 值零约束（实证）**：判官 `families_s2c.rs`（`verify_plan_write` 只查
  refill/validation 耦合）、`verifier.rs`／`conformance.rs`（零处 section 校验）、Python 镜像
  （`run_event_journal_validation.py` 零处 section 约束）——**`section="all"` 可自由携带，
  判官/镜像/schema 零改动**（「表格数据同步」纪律不触发）。
- **三操作落位**：

| 操作（裁决②语义） | 落位 | 现状 |
|---|---|---|
| 清理上下文（replace_context） | `context_manage` `mode=clear` | **在役**（0cz S2），零改动承继 |
| 清理黑板（replace_blackboard） | `blackboard_write` `op=clear`（section 单分区或 `all`） | **新增**——现状空内容校验先于 op 分支恒 exit 1，不可清空 |
| 一键清理（replace_all） | `context_manage` `mode=clear_all` | **新增**——上下文清零＋全板清空一具一次完成 |

- 现役 `context_manage` 声明 mode ∈ {compress, clear}；`blackboard_write` op ∈
  {append, replace_all}——各扩一枚枚举值，**既有调用面零回归**（缺省语义不变）。

### 3.2 D1 现场结构：build 三岔口与白名单豁免缺口

- `build_model_face` 现行短路：`cleared_before_round.is_some()` ⇒ 恒走 `build_cleared_face`
  （边界起全部逐字、无滑块钳制、不调 `hidden_message_ranges`）⇒ 清零后 T1/700K 守卫/机械压缩
  只插 marker 不落隐藏（239 批 D1 实证链）。
- `hidden_message_ranges` **已有清零区间臂**（`(0, clear_end)` 前置、不钳 slider_start——I1
  例外已在位）＋块臂 `!is_round_cleared(first_round)` 过滤——**常规路径已具备清零语义，只是不可达**。
- 三岔口：常规路径先行 `blocks.is_empty()` 早退入 `try_clone_messages_with_resident_head`
  （无块常驻头面）——**该早退不应用 hidden 区间**，故清零态无块会话若直接拆短路会漏隐藏。
- **白名单豁免缺口**：`build_cleared_face` 对 `[0, hidden_end)` 内 `[压缩白名单` 消息显式保留
  （0cz 设计 §5.3 白名单跨 clear 保留）；常规路径的 hidden 区间消费面无此豁免——clear 区间
  `[0, clear_end)` 会把白名单一并隐藏。D1 修复必须补豁免。
- 三消费点缺过滤（239 批已列）：`agent_loop.rs` 可压集合选择（≈L1286）、截断 live 过滤
  （≈L1686）、T1 可压计数（≈L2530）均用 `state()==Live`（清零块恒 Live）——把边界前已隐藏块
  误当可压/可截。

### 3.3 D2 现场结构：三方口径与守卫

- 事件/信封口径 `boundary_round1 = cur + 1 - keep`（`compact.rs` ≈L361）；marker 插入点恒
  `cur_start`（≈L420）、`clear_marker` 边界参数恒 `cur`（≈L410）⇒ keep>0 时 marker 文本写
  `cur+1`、物理隐藏 `[0, cur_start)` 含保留轮——三方分裂（239 批 D2 实证）。
- 既有守卫 `cur.checked_sub(1 + keep)` ⇒ None 早退（无可清）⇒ 到达插入点时 `cur ≥ 1+keep`，
  `cur-keep ≥ 1`，保留窗起点 `ranges[cur-keep].0 > 0`——归一后无退化（marker 不落 0 位）。
- 二次清零：`face_markers` 前向迭代后写优先＋`clear_marker_index` 取 rposition——新 marker
  落已隐藏区（keep 过大边界倒退）时两者都指向**旧** marker，信封却报成功（239 批 D5 边角）。

## §4 D1 修复设计：常规装配叠清零区间

### 4.1 装配路径重排

```
build_model_face:
  markers = face_markers(messages)                    // 单次计算下传（D4 吸收）
  cleared = markers.cleared_before_round.is_some()
  blocks  = blocks_outside_slider(...)
  if blocks.is_empty():
      if cleared: return build_cleared_face(...)      // 无块清零面（现装配原样保留，块表臂自然跳过）
      return try_clone_messages_with_resident_head(...)
  // 常规路径（有块）——清零态不再短路：
  hidden = hidden_message_ranges(...)                 // (0, clear_end) 前置臂已在位
  ...                                                 // 滑块/压缩/截断钳制照常作用于边界后内容
```

- **`build_cleared_face` 保留**，调用条件从「cleared」收窄为「cleared ∧ 无块」（开放项 O3 定案：
  保留为无块清零面专用；有块清零走常规路径）。
- 效果：清零后边界后内容照常受滑块折叠/压缩隐藏/T1 截断/700K 守卫钳制——0cz 设计 §5.3/§9
  「机制不退役、clear 不豁免兜底义务」恢复成立；滑块量尺（估算坍缩近静态）不受影响。

### 4.2 白名单豁免（跨清零保留的常规路径落地）

- 机制立场：**白名单消息（`WHITELIST_PREFIX`）在 hidden 区间内不隐藏**。实现落点 S2 按现场
  形态择一（hidden 区间展开处绕白名单分裂 ／ 渲染消费循环处豁免检查），**装配、估算、计数
  三处同口径**（对拍钉）。
- 无块清零面（`build_cleared_face`）已有同语义保留臂，两形态一致。

### 4.3 估算/计数同口径（D4 吸收）

- `estimate_model_face_tokens`／`model_face_message_count` **删除独立 cleared 分支**——与
  build 同构走常规路径（hidden 区间同源推导）＋无块清零分支；`face_markers` 每次
  公开入口恰算一次（双重扫描随分支删除消除）。
- 明确不变量：**estimate ≡ build 逐条同口径**（清零态含在内）——对拍钉固化。

### 4.4 三消费点补过滤

`agent_loop.rs` 三处可压/可截判定统一加 `!markers.is_round_cleared(b.first_round)`：
可压集合选择（≈L1286）、截断 live 过滤（≈L1686）、T1 可压计数（≈L2530）。效果：清零后
H1 窗口不再对边界前幽灵块开窗、T1 截断不再重落已隐藏块档案、freed_tokens 不再虚构。

### 4.5 钉（D1 组）

① **回落钉（核心）**：清零 marker 在场＋后边界增长构造越 T1 线 ⇒ 机械压缩/截断执行后
estimate **必须回落**（装配级对拍；修复前此钉红）；② 清零态＋白名单在场 ⇒ 有块/无块两形态
装配面均含白名单；③ 清零态无块 ⇒ 走无块清零面（marker 存根＋指针＋白名单＋边界起逐字）；④
estimate/count 与 build 三处对拍（清零态＋压缩/截断 marker 混合态）；⑤ 三消费点对清零块零命中。

## §5 D2 修复设计：marker 插入点归一＋二次清零守卫

### 5.1 插入点与参数归一

- marker 插入点＝**保留窗第一轮起点** `ranges[cur - keep].0`；`clear_marker` 边界参数＝
  `cur - keep`（marker 文本 r ＝ `cur-keep+1` ＝ `boundary_round1`）。
- 归一后四方一致：物理隐藏 `[0, marker_idx)` ＝ 轮 `< boundary_round1` ＝ 事件/信封边界 ＝
  marker 文本边界；保留轮（`cur-keep .. cur-1`）在面逐字；`clear.md` 覆盖 `[0, hidden_end)`
  恰为被清区间；dump 资格（块完全在边界前）不变即正确。
- 事件/信封/_transcript 零改动（它们本来按 boundary_round1 写）。

### 5.2 二次清零单调守卫（D5 边角吸收）

- 执行核前置校验：`markers_before.cleared_before_round` 在场且 `boundary_round1 ≤ 既有边界`
  ⇒ `Err((0, 中性说明))`（「现有清零边界已覆盖目标区间」——与「无可清零」同款中性 exit 0，
  非护栏拒绝）。埋 marker 边角随之消除。
- dump 资格补一条件：`markers_before.is_round_cleared(b.first_round)` ⇒ 已被上一清零落盘 ⇒
  跳过（单调守卫下恰为「只 dump 新增区间块」；档案幂等重写消除）。

### 5.3 钉（D2/D5 组）

① keep=2 清零 ⇒ 保留窗两轮在面逐字＋marker 文本边界＝事件边界＋保留轮**不**在 clear.md；
② keep 过大致边界倒退的二次清零 ⇒ 中性 exit 0＋零 marker＋零事件；③ 正常二次清零 ⇒ 只 dump
新旧边界之间块＋档案零重写。

## §6 清理操作多样化（replace 族，O1 载体定案立场）

### 6.1 三操作落位（§3.1 表的参数面细化）

- **清理上下文**＝`context_manage` `mode=clear`——零改动承继。
- **清理黑板**＝`blackboard_write` `op=clear`：
  - `op` 枚举扩为 {append, replace_all, clear}（缺省 append 零回归）；
  - `section` 枚举扩为 {plan, notes, findings, all}——**`all` 仅与 `op=clear` 组合合法**，
    其余组合（all＋append/replace_all）显式拒绝 exit 1（参数面零膨胀：all 不是通配写目标）；
  - `op=clear` 语义：分区模型内容清空为**空列表**（非占位条目）；分区版本计数 bump 恰 1
    （「可见内容整体替换计 1」先例同款——替换为空也是一次可见变化）；读面「（无）」已有形态；
  - journal：既有 plan_write 族顶层 `op="clear"`、`content_chars=0`、goal 槽＝
    「（清空 {section}）」预览；`section="all"` 零契约面（§3.1 实证）；判官/镜像零改动。
- **一键清理**＝`context_manage` `mode=clear_all`（§6.3）。

### 6.2 护栏不对称（裁决④落地）

- `op=clear` **无 handover**（黑板臂：journal 全量在案＋0ct 可回读，破坏半径低一量级；
  【交接摘要】即使被清，marker 存根／clear 档案／journal 三处冗余在）；信封如实回显
  「已清空 {section}（原 N 条移出板、全量留档于 journal 与会话卷）」。
- `mode=clear`／`mode=clear_all` 的上下文臂**handover 必填 ≤8K** 承继（防裸清）。

### 6.3 `mode=clear_all` 执行序（一键清理）

```
① 防裸清校验（handover 必填非空 ≤8K；缺/空/超长 ⇒ 拒绝 exit 1，零副作用）
② 黑板三分区清空（与 op=clear+section=all 同一落点函数单源；版本计数各 bump 1）
③ handover 写入清空后的 notes 分区（成为板面唯一条目，【交接摘要】标注＋盖章）
④ clear 执行核承继：回放落盘 → marker 插入 → 软水位键重置
```

- **顺序关键**：先清板后写 handover ⇒ 板面最终恰一条【交接摘要】——板面干净与黑板锚点兼得
  （若先写后清则锚点自毁）。
- 事件：**复用 `ContextCompressed` 族 `mode="clear_all"`**，payload 全量载荷（clear 既有字段＋
  `board_sections_cleared`／`board_entries_removed`）——**零新事件类型**；黑板清空作为本次
  工具调用的机械效果经该事件 params 全量入账（journal 重放即审计），**不伪造 plan_write 事件**
  （与「留痕面是工具调用事件本身」口径一致——op=clear 走 blackboard_write 才有 plan_write 事件）。

### 6.4 权限句与 guide 扩写

- `context_permission_line()` 扩为**四选择单源句**：主动压缩（mode=compress）／清零续台账
  （mode=clear，需 handover）／清理黑板（blackboard_write op=clear）／一键清理
  （mode=clear_all，需 handover）——六块提醒引用面不变（单源一次，钉面七处断言随句更新）。
- 黑板 guide 词条与【组件关系】句同步（清理黑板/一键清理入词）。

### 6.5 钉（replace 族组）

① op=clear 单分区清空（读面「（无）」＋版本计 1＋journal op=clear＋信封回显条数）；②
section=all＋op=clear 三分区全清＋版本各计 1；③ section=all＋append/replace_all ⇒ 显式拒绝
exit 1 零副作用；④ 非法 op 拒绝／journal op 字段／重写信封（D3 e2e 钉×3 并入本组）；⑤
mode=clear_all e2e（板面最终恰一条【交接摘要】＋上下文 marker 在场＋旧板条目全量在 journal＋
ContextCompressed{mode=clear_all} 载荷齐）；⑥ 权限句四选择、六块各携。

## §7 P3 项处置（D3/D5/D6/D7）

- **D3**：e2e 钉×3 并入 §6.5-④。
- **D5**：随 §5.2 吸收（单调守卫＋dump 资格补条件）。
- **D6（238 批账面勘误）**：S2 处置批档如实记（loop 读数 864→实测 867；「13 文件」＋12 个纯
  rustfmt 漂移文件＝25 文件随批提交），并在 [`238 批档`](audits/238_0CZ_S2_CONTEXT_MANAGE_IMPL_2026-10-11.md)
  尾部补一行更正注记（不改正文）。
- **D7（S4 观察项增补）**：任务漂移（裁决①零机制姿态的实测项）／清零后再触兜底行为（D1 修复
  前后对照）／二次清零形态／keep_recent_rounds 使用形态／replace 族四操作使用分布。

## §8 护栏汇总

| 护栏 | 机制 |
|---|---|
| 防裸清 | mode=clear／clear_all 的 handover 必填非空 ≤8K fail-closed；clear_all 先清板**后**写 handover（锚点保序） |
| 机制不退役 | D1 修复后清零态滑块/压缩/截断/守卫全链恢复；回落钉固化 |
| 二次清零单调 | 边界倒退 ⇒ 中性 exit 0；档案零重写 |
| 板面水位出口 | op=clear／clear_all＝板面唯一释放手段；journal 留痕「移出视野而非销毁」 |
| 契约面 | 零新事件类型；零新工具；WORK_TOOLS 三处零改动；判官/镜像/schema 零改动（section 值零约束实证） |
| 审计 | 清空/清零/一键清理均为工具调用事件链 params 全量入账，journal 重放可审计 |

## §9 批序与验收判据

- **S1 设计稿**（本稿）：O1 载体定案立场＋D1/D2 修复方案＋开放项 O1–O4 → **放行门＝用户**
  （含 O1 命名终审）。
- **S2 落码＋钉子**：§4（D1＋D4）＋§5（D2＋D5）＋§6（replace 族＋D3）＋§7（D6/D7 登记）；
  钉组＝§4.5 ⑤＋§5.3 ③＋§6.5 ⑥＋既有六钉回归全绿；读数＝orz-loop／orz-assurance 全绿＋
  fmt 净＋clippy 零新增＋触碰面三处同批对账（本批零表改动，对账＝证零改动）。
- **S3 载体重建进体**：与 0cz S3 合并同一次重建；字节判据＝mode 枚举 +clear_all＋op 枚举 +clear
  ＋section 枚举 +all＋权限句换版。
- **S4 狗粮长轮实测**：并入 0cz S4／0cy 联动轮；判据＝§7 D7 五观察项＋0cz 既有判据。

## §10 开放项（随放行裁决）

| # | 悬项 | 本稿立场 |
|---|---|---|
| O1 | **命名终审**：clear 族命名（mode=clear／mode=clear_all／op=clear）vs 用户原词 replace_* 逐字上参数面 | **clear 族**（§1：replace 语义已被 op=replace_all 占用；重命名破坏零回归；replace_* 为能力速记） |
| O2 | clear_all 的黑板清空 journal 形态：单笔 section=all vs 三笔 | **单笔**（一次工具调用＝一笔事件；版本计数三分区各 bump 与单笔事件不矛盾） |
| O3 | build_cleared_face 去留 | **保留为无块清零面专用**（§4.1）；有块清零走常规路径 |
| O4 | 权限句四选择终稿 | §6.4 立场；S2 落码按模型面文案纪律终审 |

## §11 边界与不做

- 不做黑板逐条删改（0cz §6.2 既定）；section=all 不做通配写目标（仅清空）。
- 不动机械分区／`.gsa`／plan 结构化步骤状态机（助手层）；直接改 `.gsa` 文件仍被写控拦截。
- 零新工具（面冻结例外维持 +3）；零新事件类型；零新侧车字段。
- 机械层不替模型决定「何时」清理（v8 纪律）；clear_all 不豁免在程压缩窗口义务（承继 §9 0cz）。
- 任务保留零机制（裁决①）；不对外披露（Reddit/README 沿既定口径）。

## §12 关联与关键词

[`239 批档`](audits/239_0DA_MODEL_CONTEXT_CONTROL_R2_2026-10-11.md)（立项权威＋审查发现 D1–D7 证据行号）／
[`0cz 设计稿`](MODEL_CONTEXT_CONTROL_DESIGN_2026-10-11.md)（上游权威）／
[`238 批档`](audits/238_0CZ_S2_CONTEXT_MANAGE_IMPL_2026-10-11.md)（处置对象）／
0cu（findings＝清空三分区之一）／0ct（读全开放＝板面清空可回查前提）／0ak（会话持久化）。

关键词：0da S1 设计稿、MODEL-CONTEXT-CONTROL-R2、清零后兜底失效修复、常规装配叠清零区间、
白名单豁免、marker 插入点归一、二次清零单调守卫、清理操作多样化、replace_context、
replace_blackboard、replace_all、mode=clear_all、op=clear、section=all、零新事件类型、
零新工具、护栏不对称、任务漂移观察项、索引 v4.213。
