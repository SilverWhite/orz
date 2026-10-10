# 242 批：0da S2 审查处置——跨清零边界块兜底回收＋P3×3 收口（2026-10-11）

> **用户令**：「请对审查出的全部问题进行处理」（承接 2026-10-11 对 0da S2 在役面的全面审查——
> 设计合理性／实现合理性／设计与实现符合性三轴；239 批同款流程复用于 0da 自身在役面）。
> **性质**：落码处置批——orz 子树 3 文件（model_face.rs／agent_loop.rs／compact.rs；零契约破坏：
> 判官/Python 镜像/schema/WORK_TOOLS 三处零改动；零新事件类型；零新工具）；计数不变 59；
> 未提交、未推送。
> 审查权威＝2026-10-11 全面审查报告（四发现：P2×1＋P3×3，随报告行号落锚）。

## §1 审查发现与处置对照

| # | 级 | 发现（审查报告原文摘要） | 处置（本批落地） |
|---|---|---|---|
| R1 | **P2** | **跨清零边界块被永久排除在兜底机制外**：首轮端点单判据 `is_round_cleared(b.first_round)` 用于可压集合/截断 live/T1 计数/滑块读数/块隐藏臂五处——清零后会话重生长、`blocks_outside_slider` 分区不保证在清零边界处对齐，「含边界前轮（已隐藏）＋边界后轮（可见）」的**跨边界块**被永久排除在压缩/截断外，其边界后段（≤一个 block_tokens）不可回收（D1 同类的「设计与实现共有的未推演分支」）；块表对 Live 跨边界块误标「原文」 | `FaceMarkers::is_block_cleared(first, last)`（first∧last **整块判据**）单一源：五处消费点全部收窄为整块判据（跨边界块照常参与压缩/截断回收）＋块表新增「跨清零边界（前段已移出）」如实标态。压缩/截断跨边界块安全性：边界前段已被 `(0, clear_end)` 臂隐藏（并集不变）、块档案为 clear.md 超集（无双重落档）；D5 dump 条件不受影响（跨边界块本就被 `last_round+1 < boundary` 排除） |
| R2 | P3 | **T1 可压计数把 `face_markers` 提进闭包内且置于 `b.closed` 短路之前**（241 批引入）——每块一次全量消息扫描、失去短路 | 提升至闭包外单次计算（与第一、二消费点同款） |
| R3 | P3 | **脏 marker 两段口径分裂**：`hidden_message_ranges` 内部 `clear_end` 仅按前缀自查（`clear_marker_index` 无边界行也命中），调用方前置段以 `cleared_before_round.is_some()` 为门——「有清零前缀但边界行损坏」的 marker 使主循环隐藏而前置段不隐藏，破坏「estimate ≡ build 同源推导」的干净性 | `hidden_message_ranges` 增 `clear_end: usize` 参数、调用方（build/count/estimate 三公开入口）单次 gated 推导下传——脏 marker 不虚构隐藏、公开入口恰算一次 |
| R4 | P3 | **D5 单调守卫位于 handover 校验之后**——已覆盖边界的二次清零重试若漏带 handover，先吃防裸清 exit 1 而非中性「已覆盖」说明 | 守卫（连同 `boundary_round1` 计算与 `markers_before`）前移至 handover 校验之前——「无可清」比「缺交接」更基本；零副作用不变量不变（全部拒绝仍先于黑板清空） |

## §2 钉（＋3）

1. **跨边界块钉**（`straddling_block_stays_reclaimable_and_truthfully_labeled`）：8 轮×3K/轮＋
   marker 边界第 4 轮＝首块恰跨边界（轮 1–4；block_tokens=12K 恰 4 轮/块）——①判据（首轮清零 ∧
   非整块清零）；②回收（可压读数恰 1；修复前 0）；③标态（块表「跨清零边界（前段已移出）」，
   不得「已清零」）；④压缩生效（marker 落地 ⇒ 估算回落＋边界轮〔块尾〕随块臂隐藏＋滑块内轮次
   不受影响；修复前块臂跳过 ⇒ marker 声明「已压缩」而正文在面）；⑤estimate ≡ build 对拍。
2. **脏 marker 零隐藏钉**（`dirty_clear_marker_without_boundary_hides_nothing`）：有清零前缀、
   边界行不可解析 ⇒ `cleared_before_round=None`；面/计数/估算与无 marker 基线恰差 marker 消息
   自身（修复前主循环会隐藏 `[0, marker_idx)` 而前置段不隐藏）。
3. **守卫次序钉**（`monotonic_guard_precedes_handover_validation`）：已覆盖边界的二次清零
   **漏带 handover** ⇒ 工具回执中性「已覆盖目标区间」且不含「防裸清护栏」＋零新 marker＋零
   exit 1 事件（修复前次序：先吃 exit 1）。

既有钉回归全绿（回落钉/白名单双路径钉/keep 钉/单调守卫钉/replace 族钉/权限句钉——含守卫次序
变更不破坏既有单调守卫钉：其脚本带有效 handover）。

## §3 读数

- **orz-loop**：**872/0＋竞速族 1–2 抖**——872 过（869＋3 新钉）全程稳定；失败项在
  `user_cancel_closes_pending_activations_before_run_cancelled`／
  `cancel_during_permission_await_resolves_then_terminates`／
  `cancel_mid_multi_tool_round_skips_unstarted_tools` 之间**轮换抖动**（同套件三轮复跑实证
  nondeterministic：1 败→1 败〔另一枚〕→2 败→1 过 1 败），均为 30ms 取消窗竞速家族
  （209 批在案「user_cancel 竞速族」；241 批 stash 定界先例同款），与本批触碰面（装配/
  过滤/清零核）零交集——环境性，不在本批处置。
- **orz-assurance**：**301/0**。
- **fmt**：触碰面净（`cargo fmt -p orz-loop -- --check` 过）。
- **clippy**：触碰行零警告（`--all-targets` 警告位置逐一核对——agent_loop/model_face/compact
  本批触碰区间零命中；存量警告面与 HEAD 先存同分布）。

## §4 批序

→ **S3 载体重建进体**（与 0cz S3 合并同一次重建，待载体重建令；字节判据不变＝mode 枚举
+clear_all＋op 枚举 +clear＋section 枚举 +all＋权限句换版＋声明描述扩写）→ **S4 狗粮长轮
实测**（并入 0cz S4／0cy 联动轮；D7 五观察项）。本批为纯审查处置，无新增 S 项。

## §5 台账

- 本档：`docs/audits/242_0DA_S2_REVIEW_DISPOSAL_2026-10-11.md`。
- BACKLOG：本批指针＋计数行（计数不变 59）＋`0da` 批序行 S2 处置注记＋P1 总览＋入口行。
- TODO：计数行＋P1 路由＋`P1-0da` S2 处置勾选项。
- BACKLOG 第二卷：§1.188。
- 索引：头行 v4.214 → **v4.215**；§6 `0da` 条目 S2 处置注记。
- 门禁 `scripts/check_repository.py` ⇒ 落账后重跑（预期 errors=1＝orz 子树 dirty）。

## §6 关联与关键词

[`241 批档`](241_0DA_S2_CONTEXT_MANAGE_R2_IMPL_2026-10-11.md)（处置对象）／
[`设计稿`](../MODEL_CONTEXT_CONTROL_R2_DESIGN_2026-10-11.md)／[`240 批档`](240_0DA_S1_DESIGN_2026-10-11.md)／
[`239 批档`](239_0DA_MODEL_CONTEXT_CONTROL_R2_2026-10-11.md)（立项＋审查流程权威）／
209 批档（user_cancel 竞速族在案）。

关键词：242 批、0da S2 审查处置、跨清零边界块、is_block_cleared 整块判据、兜底回收、
块表跨清零边界标态、脏 marker 不虚构隐藏、hidden_message_ranges 参数化、单调守卫前移、
T1 计数扫描提升、872/0、竞速族抖动、301/0、计数不变 59、索引 v4.215。
