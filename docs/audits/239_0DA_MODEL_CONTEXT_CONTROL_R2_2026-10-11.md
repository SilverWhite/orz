# 239 批：0da 立项——0cz 全面审查处置＋清理操作多样化 replace 族（MODEL-CONTEXT-CONTROL-R2）（2026-10-11）

> **用户令（三段，2026-10-11）**：①「最初的用户任务得留下来才行……现在的清零机制是类0cw的那种形式，
> 黑板和台账不清零，有plan应该问题不大，先不额外对任务保留做机制处理试试看」②「我考虑应该给模型
> 多样化的选择，replace_all/replace_blackboard/replace_context应该分开给，便于模型按需使用，一键清理/
> 清理黑板/清理上下文都按需选择」③「本轮审查出的全部问题都得处理，无论大小。请立项一个问题处理批次」。
> **性质**：立项登记批——零源码；计数 **58 → 59**（新立项 0da）；未提交、未推送。
> **审查权威**：2026-10-11 对 0cz S2 在役面（[`238 批档`](238_0CZ_S2_CONTEXT_MANAGE_IMPL_2026-10-11.md)）
> 的全面审查（设计合理性／实现合理性／设计与实现符合性三轴），orz 子树 13 文件实现 diff 逐文件实读＋
> fmt 漂移甄别＋恢复/串行链路核实＋测试复跑。

## §1 canonical ID 与定位

- **canonical ID**：`MODEL-CONTEXT-CONTROL-R2`（**0da**，P1，58 → 59；0cz 的审查处置＋扩展轮，
  R2 后缀沿 0bj-R2/R3 先例）。
- **一句话定位**：把 0cz S2 在役面审查出的**全部问题（无论大小）**全量处置——纲＝**P1 修复：清零
  生效后兜底阶梯与滑块在投影层整体失效**（设计稿 §5.3/§9「机制不退役」的符合性破口）——并按用户
  裁决把**清理操作多样化**（清理上下文／清理黑板／一键清理三选择分开给）作为扩展主项同批设计；
  任务保留按裁决①登记为**零机制姿态**（S4 观察项）。

## §2 用户裁决（三段转录与界定）

| # | 裁决 | 界定 |
|---|---|---|
| ① | **任务保留不另做机制**：清零＝类 0cw 重置形态（黑板＋台账不清零、有 plan），原任务 prompt 出窗为**既定设计姿态**，先不额外做机制试试看 | 落点＝S4 观察项「任务漂移」；审查报告原「待观察项」升格为设计姿态，零机制动作 |
| ② | **清理操作多样化**：`replace_all`／`replace_blackboard`／`replace_context` 分开给——**一键清理／清理黑板／清理上下文**按需选择 | 语义界定：`replace_context`＝清理上下文（现 `mode=clear` 语义承继）；`replace_blackboard`＝黑板三分区（plan.model_notes/notes/findings）**板面清空**（journal/会话卷留痕不变）；`replace_all`＝**一键清理**（上下文＋黑板全清）。命名与既有 `blackboard_write op=replace_all`（整节重写）撞名——消歧为 S1 开放项 O1 |
| ③ | **审查处置全量**：本轮审查出的全部问题都得处理，无论大小 | §3 D1–D7 全项入批序，处置计划随 S1 设计稿定稿、S2 落码 |

## §3 审查底座（2026-10-11 全面审查发现全项）

**审查读数**：orz-loop **867/0**（3 ignored；批档记 864，见 D6）＋orz-assurance **301/0**（与批档一致）。
**甄别记录**：工作树 25 脏文件中 12 个为纯 rustfmt 漂移（去空白比对逐一核实：console/console_exec/
acaf/delivery/gateway-fake/serp/disposition/controller_test_support/credential/activation/lif_replay/
rli_forecast_probe），0cz 实质面恰 13 文件与批档一致。

| # | 级 | 发现 | 证据 | 处置方向 |
|---|---|---|---|---|
| D1 | **P1** | **清零生效后兜底阶梯与滑块整体失效**：`build_model_face` 见清零 marker 即短路 `build_cleared_face`（边界起全部逐字、无滑块钳制、不调 `hidden_message_ranges`）⇒ 清零后 T1/700K 守卫/机械压缩只插 marker 不落隐藏——面估算单调上涨直至 provider 拒绝；与设计稿 §5.3「H1/T1 沿越线重武装自然生效」§9「机制不退役」相悖（**设计与实现共有的未推演分支**：兜底收缩力全在常规装配路径，清零态绕行） | `model_face.rs` build_model_face/build_cleared_face；`agent_loop.rs:1666`（truncate 只插 marker）、`:2686`（守卫逐轮原始条件） | 常规装配叠前置清零区间 `[0, marker_idx)`（I1 例外已在 `hidden_message_ranges` 登记），边界后内容照常受滑块/压缩/截断钳制；钉＝「清零后越 H1/T1，面估算必须回落」 |
| D2 | **P2** | **`keep_recent_rounds > 0` 三方不一致**：事件/信封口径 `boundary_round1 = cur+1-keep` 与 dump 资格按它；但 marker 插入点恒为当前轮起点（`messages.insert(cur_start,…)`）、marker 边界参数恒 `cur`（文本写 `cur+1`）⇒ 物理投影把「保留轮」一并隐藏、marker 文本与信封边界互相矛盾、「保留轮」不进任何档案（块级与 `-clear.md` 都止于保留窗前）。keep=0（默认）三者恰好重合，六钉全绿 | `compact.rs` L361/L410/L420 | marker 插入点移至保留窗第一轮起点 `ranges[cur-keep].0`（插入点/参数/boundary_round1 归一，二次清零保持单调）或砍参数——S1 定案（立项立场＝修好保留，合裁决②「多样化选择」哲学）；该参数与单边界 marker 的不相容属设计稿 §11 内部矛盾 |
| D3 | P3 | **`op=replace_all` 派发面无端到端钉**：钉④只落 blackboard 单元级（版本计数＋渲染）；非法 op 拒绝（exit 1）、journal 顶层 `op` 字段、重写信封文案三条派发分支零覆盖 | `tool_run.rs:1615-1730` | 补 e2e 钉×3 |
| D4 | P3 | `estimate_model_face_tokens`／`model_face_message_count` 清零分支连续两次 `face_markers` 全量扫描（`estimate_cleared_face_tokens` 内再扫一次） | `model_face.rs:953/1080` 一带 | 消除冗余扫描（微） |
| D5 | P3 | 二次清零：前次已按块落盘的 Live 块重进 dump 名单（档案幂等覆盖、纯浪费）；keep 过大时新 marker 落进已隐藏区被静默埋掉而信封仍报成功 | `compact.rs` dump 循环 | dump 资格补「未被先前 clear 覆盖」判断；埋 marker 边角随 D2 修复消解后补钉 |
| D6 | P3 | **238 批账面两处**：①「orz 子树 13 文件」vs 工作树 25 文件（12 个纯 fmt 漂移未记账，随本批提交会一并进版本）；②loop 读数批档 864 vs 实测 867（3 ignored 一致、零失败，疑记账时点差异） | `git status`＋去空白比对＋复跑 | 批档补记＋读数勘误（随 S2 处置批或提交批落账） |
| D7 | P3 | **S4 观察项增补**：任务漂移（原任务 prompt 出窗——裁决①既定姿态）；清零后再触兜底的实际行为（D1 修复前后对照）；二次清零形态；`keep_recent_rounds` 使用形态 | — | 并入 0cz S4 判据面 |

**审查确认无误面**（如实记）：防裸清 fail-closed 顺序（拒绝路径黑板零写入有钉）；marker 只插入 I3；
I1 例外三条件落地；恢复重建链路（`is_restore_retained_block` 收录清零前缀、sidecar 往返可重建）；串行面
（不在 `PARALLEL_READ_TOOLS`，clear 对 `messages` 无并发竞态）；三处 WORK_TOOLS 同批；单源常量；
ReadOnly 分类先例论证；§4/§5.1/§5.2/§5.4/§6/§7/§8/§10 高度符合（六块提醒>四块、整段现场档>仅块级
为方向正确的超集）；§9「机制不退役」一条不符（D1）。

## §4 扩展主项：清理操作多样化（replace 族）设计立场

- **三操作语义**（裁决②界定，S1 细化）：`replace_context`＝清理上下文（`mode=clear` 语义承继，含
  handover 防裸清）；`replace_blackboard`＝黑板三分区**板面清空**（现状不可清空——`blackboard_write`
  空内容校验先于 op 分支恒 exit 1，`tool_run.rs` ≈L1540；replace_all 只能 N→1 不能→0）；`replace_all`＝
  一键清理（上下文＋黑板全清）。
- **哲学自洽**：0cz §0「黑板＝它的笔记」——全量编辑权逻辑上包含清空；「移出视野而非销毁」黑板同构
  （journal/会话卷逐字在案，0ct 读全开放可回查）。板面 10MiB 软水位目前**无任何模型可用释放手段**
  （replace_all 只能瘦身到 1 条），清空是唯一出口——0cz「上下文自管理」同一哲学的下半篇。
- **护栏不对称（立项立场，S1 定案）**：`replace_context` 与一键清理的上下文臂承继**防裸清 handover**；
  `replace_blackboard` **无需同级护栏**（journal 全量在案＋可回读，破坏半径低一量级；【交接摘要】即使被
  清，marker 存根／clear 档案／journal 三处冗余在）。三分区一致放行、不做分区歧视（journal 即审计）。
- **开放项 O1（载体形态与命名消歧）**：`replace_all` 与既有 `blackboard_write op=replace_all`（整节重写）
  撞名——候选形态＝`context_manage` mode 枚举扩展 vs `blackboard_write` op 枚举扩展 vs 独立工具
  （面冻结例外 +4？）；S1 对注册表现场盘点后定案，零回归为底线（既有 op=append/replace_all 调用面不动）。
- **空分区读面**已有现成形态（「（无）」，`blackboard.rs` 测试在案）；版本计数 bump、journal `op` 字段、
  PULL 增量承接全部继承 0cz S2 已落的面。

## §5 批序

→ **S1 设计稿**（D1/D2 修复方案定稿＋replace 族载体形态/命名消歧/护栏定案＋开放项列明）→
**S2 落码＋钉子**（审查处置 D1–D7 全项＋replace 族；钉组＝D1 回落钉＋D2 keep 钉＋D3 e2e 钉×3＋
replace 族清空/护栏钉＋D5 边角钉）→ **S3 载体重建进体**（**与 0cz S3 合并同一次重建**；字节判据合并
登记）→ **S4 狗粮长轮实测**（并入 0cz S4／0cy 联动轮；观察项按 D7）。

## §6 依赖与边界

- 0cz S3 尚未进行：0da S2 与 0cz S3 合并进同一次载体重建，避免双重建。
- 真机面依赖 0cy 卫生前置不变（0cz 既定时序约束顺延）。
- 任务保留零机制（裁决①）；不对外披露（Reddit/README 沿既定口径）。
- 本批零源码。

## §7 台账

- 本档：`docs/audits/239_0DA_MODEL_CONTEXT_CONTROL_R2_2026-10-11.md`。
- BACKLOG：计数行（**59**）＋本批指针＋P1 节 `0da` 专节＋优先级总览＋开放项清单＋`0cz` 批序行注记。
- TODO：计数行＋P1 路由（0da 入列；顺带把路由行 `0cz` 状态句同步为 S1/S2 达成〔237/238〕——238 批
  未更的账）＋`P1-0da` 节。
- BACKLOG 第二卷：§1.185。
- 索引：头行 v4.211 → **v4.212**；§6 `0da` 条目；§8 pending 桶同步。
- 门禁 `scripts/check_repository.py` ⇒ 落账后重跑（orz 子树 dirty＝S2 未提交中间态，预期报错一项）。

## §8 关联与关键词

[`238 批档`](238_0CZ_S2_CONTEXT_MANAGE_IMPL_2026-10-11.md)（审查对象）／
[`设计稿`](../MODEL_CONTEXT_CONTROL_DESIGN_2026-10-11.md)（0cz S1 权威）／
[`237 批档`](237_0CZ_S1_DESIGN_2026-10-11.md)／[`236 批档`](236_0CZ_MODEL_CONTEXT_CONTROL_2026-10-11.md)／
0cu（findings 分区＝replace_blackboard 三分区之一）／0ct（读全开放＝板面清空可回查前提）／
193 批（审查处置批先例）。

关键词：239 批、0da 立项、MODEL-CONTEXT-CONTROL-R2、0cz 全面审查、清零后兜底失效、
build_cleared_face、keep_recent_rounds 不一致、清理操作多样化、replace_context、replace_blackboard、
replace_all 一键清理、黑板板面清空、任务保留零机制、58 → 59、索引 v4.212。
