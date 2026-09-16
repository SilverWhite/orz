# 0ah v8 滑块上下文**实现更正批**（模型面投影层／块表／新阶梯／退役／按块回放）

> **类型**：实施批回执（执行事实与读数；设计语义以 [`v8 设计稿`](../CONTEXT_SLIDER_V8_DESIGN_2026-09-16.md) 为准）。
> **性质**：**勘误**（记录错误 → 实现错误）的实现侧更正；**不写「翻转／取代」叙事**。
> **上位**：BACKLOG **0ah** / TODO **P1-0ah**；索引 `AUTH-CONTEXT-DYNAMIC-SLIDER`（v8）。
> **源冻结基线**：orz `9b822914`（0.5.4 载体重建提交）；本批改动**尚未提交**。
> **用户指令（2026-09-16）**：「请开始**实现更正批**：模型面投影层（本地面全量零覆盖）、
> 块表与块号去重、新阶梯 H1/T1、退役 192K rhythm 与 256K 兜底、按块回放面」。
> **登记归属**：ADR 转录／索引／BACKLOG／TODO 的正式登记另立批（本件只作实施回执与证据）。
>
> **审查回执（2026-09-16）**：本批已过**只读审查**
> [`0ah v8 实现更正批只读审查`](0AH_V8_IMPLEMENTATION_REVIEW_2026-09-16.md)——结论＝**有条件不通过**
> （P0×1：T1 硬截断吞掉主滑块；P1×2：分块表每轮重写打掉成本前提、收尾／恢复仍覆盖本地面；
> P2×7／P3×1）。审查**未改动任何实现代码**；三条处置为后续动作的前置（清单见审查件 §8）。
>
> **审查处置批（同日落码，见 §8）**：审查 R-1…R-11 **全部处置**（无遗留到下一批）——
> 三处口径级修复＋新钉子＋设计面同步（设计稿 §2/§3/§5/§6/§7/§8/§10/§11/§12 补录、
> ADR-0010 §14.69 补录、成本记录补注）。**用户同日三条裁定**：①守卫降值 ＋ T1 重新武装
> （双管齐下）；②H1 打断点确认＝**越线后的下一个 loop-top**（越线那批工具结果已入窗口、
> 下一次请求组装之前），H1 随 T1 一起重新武装；③「该修的都修，按最佳修复」。

## 1. 交付面（用户五条指令 → 落点）

| # | 指令 | 落点（实现事实） |
|---|---|---|
| ① | **模型面投影层（本地面全量零覆盖）** | 新模块 `orz-loop/src/model_face.rs`（877 行，含单测）：`build_model_face` 装配 `前置 ＋ 固定指针 ＋ 分块表 ＋ D4 机械段 ＋ 各分块 ＋ 主滑块 x ＋ 尾部`；`agent_loop` 请求装配改走投影层（窗口轮与普通轮**同形**——v8 的模型面本来就含全部携带内容）。**压缩不再 `drain` `messages`**：按块压缩 / T1 截断只往会话**追加** marker（`[前文上下文已压缩 v0.4-分块压缩｜v0.4-分块截断]`，restore-retained ⇒ 恢复后块状态由 marker 反解重建，**无新侧车字段**）。 |
| ② | **块表与块号去重** | `model_face::blocks_outside_slider`（编号**锚在会话起点** ⇒ 跨轮次稳定）＋ `render_block_table`（模型面可见 `[上下文分块表 v0.1]`，注册进 `is_injected_block_text`）＋ `face_markers`／`parse_marker_blocks`（块号 ↔ 状态幂等反解：同一块在会话内不重复计数；**只有「已闭合」块可压**——末块仍在增长不压，避免摘要与正文漂移）。 |
| ③ | **新阶梯 H1/T1** | `context_scale.rs` 重写：`DEFAULT_LADDER = 192/224/256/288K 软提醒 → **320K 硬提醒（打断＋开压缩窗口）** → **500K 硬截断**`；量尺＝**模型面估算**（`model_face_estimate`）；每档每会话一次、不 rearm。`agent_loop` loop-top：软提醒注入 → H1 注入（硬提醒＋分块表＋窗口任务）并开窗 → T1 **把主滑块以外的全部分块移出模型面**（逐字落盘可按块回放 ＋ 告知块「已截断 N 块／可按块回放／任务无需中止」＋ `anomaly=hard_context_truncated_blocks`）。同迭代同时越 H1 与 T1 时**只截断**（提醒已无意义）。 |
| ④ | **退役 192K rhythm 与 256K 兜底** | `compact.rs`：删 `safety_tokens`／`slider_resident_tokens`（L）／`slider_rhythm_buffer_tokens`／`rhythm_tokens()`／`rhythm_tokens_override`／`hard_context_tokens`／`window_upload_cap_tokens`／`context_scale_milestones` 与五条 v7 env（`ORZ_SLIDER_WINDOW_TOKENS`／`ORZ_SLIDER_RESIDENT_TOKENS`／`ORZ_SLIDER_RHYTHM_BUFFER_TOKENS`／`ORZ_CONTEXT_SCALE_HARD_TOKENS`／`ORZ_CONTEXT_SCALE_WINDOW_CAP_TOKENS`——**构造时不再读取**，同 `ORZ_LADDER_*` 先例显式作废）；`run_template_compact` 删 `fallback` 第二段截断（`compact_messages` 截断至 `recovery_target_tokens`）与语义轨 drain 分支；loop-top 删 rhythm／fallback／950K 三条触发链。**机械轨按内容策略照常工作**（结构化台账行＋指针＋compaction 存档＋事件，作用于模型面），但**不再以「总量超线」为由压缩模型面**。 |
| ⑤ | **按块回放面（形态①，零新工具）** | 压缩/截断一个块 ⇒ 逐字原文落盘 `{cwd}/.gsa/compaction/blocks/<tag>-block-<k>.md`（块头附陈旧性标注「内容截至轮次 N；文件可能已变更，编辑前须新鲜读取」）；marker 与分块表行都带「完整内容见〈路径〉，用 `read_file` offset/limit 分页」指针——**不动 8 工具面**。回放块状态幂等（去重键＝块号，再次压缩同一块 ⇒ NoOp）。 |

**新增 env（v8 命名；最终命名留登记批）**：`ORZ_MODEL_FACE_SLIDER_TOKENS`（主滑块 x，默认 160K 估算）、
`ORZ_MODEL_FACE_BLOCK_TOKENS`（分块 y，默认 32K）、`ORZ_MODEL_FACE_GUARD_TOKENS`（上限守卫，默认 1.10M）。

**模型面措辞（设计 §3 纪律：「不用内部词汇」）**：固定指针文案由 v7 的「【历史折叠】…窗口只保留
最近若干完整轮」改为「【上下文分块】…旧内容按**分块**累积、被压缩/截断的分块可按块回放」；
各提醒／告知块统一说「**当前上下文窗口**」「**工作现场**（你最近工作的连续轮次）」「**分块**」，
不再出现「滑块／主滑块／驻留带／折叠」等内部沿革词（Rust 注释与设计稿仍用内部分区术语，两套词汇各归其位）。

## 2. 量尺对照（设计 §3.1「量尺纪律」）

| 判定 / 动作 | v8 取尺 | 代码位 |
|---|---|---|
| 阶梯（软提醒／H1／T1） | **模型面估算**（投影层视图 chars/2） | `model_face::model_face_estimate` ← loop-top |
| 分块 / 主滑块切分 | 同一把估算尺（与 v7 的 `bridge_cut` 同源：整轮对齐） | `model_face::blocks_outside_slider` / `bridge_cut` |
| 上限守卫（1.10M 异常保险） | 模型面估算；越线 ⇒ **强制截断到线上** | `truncate_model_face_blocks` |
| 台账摘要行／compaction 存档／事件读数 | 模型面估算（`trigger_tokens`／`estimated_tokens_after`／`messages_*` 均按**模型面**口径；本地面一条不删） | `compress_blocks_now` |
| provider 实测 prompt token | **不再参与任何判定**（v7 的 rhythm／fallback 视图尺随勘误退役；实测值仍照旧入 `model_output` 事件） | loop 内变量一并删除 |
| 收尾／恢复预检 | **未变**（仍走本地全量估算与既有阈值，见 §7 待裁决项） | `controller.rs` / `compact_messages` |

## 3. 新钉子（先写后跑）

| 钉子 | 断言（要点） |
|---|---|
| `model_face::tests::blocks_are_numbered_from_the_conversation_start_and_stay_stable` | 块号锚在会话起点、追加轮次后既有块号／首轮不变；末块未闭合 |
| `model_face::tests::no_blocks_when_the_whole_conversation_fits_the_slider` | 无分块 ⇒ 模型面＝原文（字节级）；量尺＝全量估算 |
| `model_face::tests::model_face_hides_compressed_blocks_and_keeps_the_slider_verbatim` | 已压块的消息移出模型面、marker 留在模型面、**本地面只多一条 marker** |
| `model_face::tests::block_table_lists_state_and_replay_pointer` | 分块表列状态＋回放路径＋run |
| `model_face::tests::markers_round_trip_block_numbers_and_never_get_hidden` / `truncation_marker_marks_blocks_idempotently` | marker ↔ 块号往返、幂等；历史 marker（v0.1–v0.3）不参与反解 |
| `context_scale::tests::ladder_fires_each_tier_exactly_once_and_in_policy_order` | 六档每档一次、T1 单独一次、顺序与 tier 正确 |
| `context_scale::tests::reminder_blocks_carry_the_model_face_reading_and_state_the_truncation_line` / `reminder_blocks_are_registered_injected_text` | 文案带**模型面读数**＋宣告 T1 口径＋I5 声明；全部注册为注入文本 |
| `context_scale::tests::summary_block_is_extracted_with_optional_block_selection` | `[SEMANTIC_SUMMARY]` 识别 ＋ `压缩块: 2-4` 区间指令 |
| `agent_loop::tests::first_block_reminder_fires_exactly_once_after_the_first_block` | A4 改锚「首个分块形成」；每 run 恰一次；模型面带固定指针＋分块表 |
| `agent_loop::tests::model_summary_block_replaces_the_region` | 按块压缩：`mode=model_summary`／`reason=model_selected`；marker `v0.4-分块压缩`＋`已处理分块: 1`＋四项定位指针＋回放路径；**回放档案落盘**；被压块原文移出模型面 |
| `agent_loop::tests::block_compaction_is_idempotent_by_block_number` | 同一块二次点名 ⇒ 恰一条 `context_compressed`；分块表状态「已压缩」 |
| `agent_loop::tests::t1_truncation_moves_blocks_out_of_the_model_face_and_keeps_the_local_face` | T1：`anomaly=hard_context_truncated_blocks`；告知块（可回放／任务无需中止／I5）；回放档案含陈旧性标注；模型面里无被截断块工具结果；**持久化会话仍含逐字原文**＋截断 marker |
| `agent_loop::tests::model_face_guard_forces_truncation_above_the_safety_line` | 1.10M 守卫 ⇒ `anomaly=hard_context_guard_truncated_blocks`＋守卫告知块 |
| `agent_loop::tests::block_compaction_reports_archive_write_failure_explicitly` | `.gsa/compaction` 落点被占 ⇒ 压缩照常执行＋事件 `archive_write_failed=true`（失败绝不静默） |
| `agent_loop::tests::in_window_oversized_result_is_pointerized_until_under_the_line`（改写） | 主滑块之外**无分块**时改走窗口内超大结果指针化（0ah §10.10 裁定保留），模型面真降线、配对字段不动 |
| `agent_loop::tests::compression_window_*`（4 钉，改驱动件） | H1 开窗的面／语义／收口 `model_participated` 三态与 budget break 前收口（驱动件＝`v8_test_ladder(1_000, 1e9)`＋短 prompt） |
| `agent_loop::tests::context_scale_watermark_is_session_level`（改写） | 六档键预置 ⇒ 零提醒／零事件；水位键原样 |
| `agent_loop::tests::final_answer_semantic_summary_is_consumed_at_the_run_tail`（改写） | run 尾按块压缩消费终止轮摘要 |

**随勘误退役的钉子（删除，如实登记）**：`context_compact_summary_fires_at_mid_task_gap`、
`context_compact_requires_measured_trigger`、`context_compact_respects_min_rounds_cooldown`、
`context_compact_safety_trigger_bypasses_cooldown`、`context_compact_mechanical_mode_zero_model_calls`、
`context_compact_guard_failure_retries_then_forces`、`fold_state_advances_once_and_prefix_stays_stable`、
`fold_append_failure_disables_fold_and_keeps_session_going`、
`fold_state_resets_after_mechanical_compaction_and_archive_keeps_pointer`、
`p2_14_s2_rhythm_fallback_session_end_serial_v03_marker`、`context_compact_defaults_follow_v1_14_review`、
`slider_tokens_parse_rule`、`fallback_second_stage_truncation_accounts_total_rounds`、
`hard_context_fallback_fires_on_the_actual_context_scale`、
`hard_context_failure_with_dropped_rounds_reports_the_count`、
`hard_context_failure_intercepts_and_tells_the_model_without_stopping`、
`context_scale_900k_escalates_with_the_actual_reading`（改写为 `context_scale_watermark_*` 水位钉）、
`context_scale_500k_is_a_pure_reminder_without_a_window`、
`window_over_upload_cap_degrades_instead_of_opening_a_window`（守卫改为强制截断）、
`compression_window_request_loads_the_pending_region`（v8 模型面恒含全部携带内容 ⇒ 前提消失）、
`summary::tests::summary_archive_write_failure_is_reported`（旧触发链驱动件）。等价覆盖由 §3 新钉子承担。

## 4. 实跑读数（本机，2026-09-16）

- `cargo test -p orz-loop --lib` → **807 passed / 0 failed / 3 ignored**（R-12③ 处置批 +2 钉；
  处置批前：805/0/3，再前：794/0/3 ＋ 9 失败＝旧机制钉子）。
- `cargo test -p orz-host --lib -- --test-threads=1` → **328 passed / 0 failed / 5 ignored**（与 0.5.4 基线一致）。
- `cargo test -p orz-assurance` → **229** lib ＋ fixtures 全绿（含 Rust↔Python verdict parity）。
- `cargo fmt --all -- --check` 干净。
- `cargo clippy -p orz-loop -p orz-host --lib` → orz-loop **50** 条（基线 52 ⇒ **净 −2，零新增**；
  R-12③ 处置批后复跑仍 50）、
  orz-host **12** 条（基线 12 ⇒ 持平）。（`protoc` 缺失导致 clippy 需 `PROTOC=D:\CLI\orz\bin\protoc.exe`，
  codegen 侧构建只有该环境依赖，与本批无关。）
- `python -m pytest runtime/tests -q` → **361 passed / 1 failed**（唯一失败＝既有无关红灯
  `test_v02_all_51_event_types_covered`：断言 55 vs registry 65，本批未触碰事件注册表）。
- `python scripts/check_repository.py` → 唯一 error ＝ `orz submodule working tree is dirty`（本批未提交所致；
  其余检查全过）。
- **契约面（随批同步）**：`runtime/context-compressed-event-payload-v0.2.schema.json` 与
  `runtime/mechanical-audit-update-event-payload-v0.2.schema.json` 的 description 按 v8 改写
  （**闭枚举值不变**：`rhythm`／`fallback`／`context_scale` 保留为历史回放值，生产零写入；
  新增 anomaly 词表 `hard_context_truncated_blocks`／`hard_context_guard_truncated_blocks`／
  `hard_context_result_pointerized`／`hard_context_guard_result_pointerized`）；
  `assurance/run_event_journal_validation.py` 与 `orz-assurance/src/journal/families_s2c.rs` 的判定规则**不变**、注释同步。

## 5. 登记归属（本批未做，随登记批）

1. ADR-0010 §14 转录 v8 机制（含勘误说明）——设计稿 §13 已列该清单。
2. 索引 `AUTH-CONTEXT-DYNAMIC-SLIDER`／`GAP-DYNAMIC-CONTEXT-SLIDER`、BACKLOG／TODO 0ah 的
   「S1 五连批＝按错误记录落码」叙述与批序改写（v8 实施 → 判据读数）。
3. `orz_source_manifest.sha256`：**新文件 `model_face.rs` 当前为未跟踪状态**，manifest 生成器按
   `git ls-files` 取件 ⇒ 须在 orz 子模块提交后重跑（预期 1456 → 1457 条）。
4. 提交前需用户确认（本批未提交；父仓与 orz 子模块均待放行）。

## 6. 未核项（如实标注）

1. **无真机 run**：本批全部读数来自单测与既有 fixtures；0ai 量级/长会话下的
   模型面工作点、H1 是否真被模型消费（产出摘要块）、T1 的触发次数与回放使用率**未测**（属批序 ②）。
2. **成本**：hit／miss、重写税均为设计 §9 与[成本重算记录](../CONTEXT_SLIDER_V8_COST_RECOMPUTATION_2026-09-16.md)
   的解析值，**未实测**。
3. **1.10M 守卫与 T1 的叠加真机路径**未跑（单测分别覆盖）。
4. **模型面内指针化的阈值**（`OVERSIZED_TOOL_RESULT_CAP_TOKENS` = 8K 估计）与「主滑块之外无分块时才启用」
   的边界未在真机校准。
5. **`.gsa/compaction/blocks/` 的可见性**：与既有台账指针同族——`.gsa` 内部区读需经两段门（首读通知 → 二读放行）。
   **2026-09-16 裁定（审查 R-12⑤）：不扩白名单窗口**——读码核证通知给过一次后（`access_state.json`
   卷级持久化）内部区后续读**直接放行**，回放面在门语义下本就可达；扩窗口等于把压缩存档从
   「通知后开放」改成「免通知恒开放」，无功能增益而动 `AUTH-GSA-SESSION-VOLUME` 授权面。
6. **回放块的注入预算**（50K 单次）沿用 `ORZ_MAX_INJECT_TOKENS_PER_ROUND` 既有纪律，**未在本批新写钉子**。

## 7. 边界与待裁决（本批发现，未擅自扩张）

1. **收尾／恢复预检仍会覆盖本地面**：`controller.rs` 的 `session_end` 机械模板压缩（会话关闭前
   `drain` ＋ marker 钉进侧车）与 D2-2 恢复预检（`compact_messages` 截断）**未在用户五条指令与
   设计 §10 退役清单内**，故本批**按原样保留**。但二者与 v8 §7「本地面＝单对话全量、不再因压缩
   drain 出会话本体」及判据「本地面完整性 100%」存在**口径冲突**：会话关闭后侧车（进而会话归档包）
  不再逐字全量。**建议**随登记批一并裁决（选项：退役 session_end 压缩／改为「归档前另存全量副本」
   而不动侧车／维持现状并如实标注判据口径）。
   **处置补录（2026-09-16）**：本条已在**同日审查处置批**按 v8 §7 口径落地——见 §8 **R-3**：
   主车道收尾压缩与 D2-2 恢复预检**双双退役**（`context_recovery_truncated` 生产零写入）。
2. **机械轨的独立触发面归零**：v8 下机械结构化压缩只在「模型自压（按块）」与「检索/grill 车道＋
   会话收尾」两处发生；机械层不再有任何以「总量超线」为由的自主触发（＝用户裁定）。
3. **H1 的「打断」形态**取「注入 ＋ 开 ≤3 轮压缩窗口（仅 `blackboard_write`）」，窗口收口
   **不做机械兜底压缩**（未产出摘要 ⇒ 如实落账 `model_participated=false`）。
4. **v7 遗留代码**：`action_ledger` 的折叠／推进族（`advance_fold`／`build_request_view`／桥渲染）
   在 v8 主车道**生产零调用**（`run_template_compact` 的检索／收尾路径仍传 `fold_state`，
   恒为未折叠态），本批**保留未删**（删面涉及检索车道与多处测试，属独立批；如实登记为待清理项）。

## 8. 审查处置批（2026-09-16，同日落码；用户令「该修的都修，按最佳修复」）

处置对照（详表见[`审查件 §9`](0AH_V8_IMPLEMENTATION_REVIEW_2026-09-16.md)）：

| 审查项 | 处置（实现事实） |
|---|---|
| **R-1（P0）** T1 吞主滑块 | 三处口径修复：① 未闭合末块 `last_round` 改记**它真正累计到的最后一轮**；② 压缩/截断**只点名已闭合块**（残段留在窗口内）；③ `hidden_message_ranges` 双护栏＝**未闭合永不隐藏** ＋ 区间**钳到主滑块起点**。新钉子：`model_face::open_residual_stops_before_the_slider_and_is_never_hidden`；`t1_truncation_*` 补「主滑块（call-r2）仍在场」断言 |
| **R-2（P1）** 分块表每轮重写 | 表**移出前缀、落窗口尾部**（逐轮追加）：两次压缩/截断之间前缀字节稳定（I6 恢复），表变化只落尾部 |
| **R-3（P1）** 收尾／恢复覆盖本地面 | 主车道**收尾压缩退役**（`session_end` 调用删除）＋**D2-2 恢复预检退役**（`recovery_*`／`compact_messages`／`context_recovery_truncated` 生产零写入）；首请求体积约束改由 loop-top 阶梯/守卫在发请求前承担。新钉子：`compact::main_lane_session_end_keeps_the_local_face_verbatim`、`controller::restore_keeps_the_local_face_verbatim_and_projects_the_model_face` |
| **R-4（P2）** marker 文案矛盾 | `LocatorPointers.local_face_full`：按块路径置真 ⇒ 定位文案改口「sidecar 含被处理分块逐字原文／存档只存摘要与指针／按块档案＝逐字」 |
| **R-5（P2）** T1 落盘失败静默 | 档案/台账失败**如实落账**（`archive_write_failed=`／`ledger_write_failed=`）＋写进告知块＋进截断记录存档；新钉子 `agent_loop::t1_archive_write_failure_is_reported_to_the_model_and_the_journal`（复用 `BlockedArchiveHost`） |
| **R-6（P2）** 块表缺两键 | marker 增 `台账定位: 块#k [seq] a-b`（`parse_marker_ledger_seqs` 反解）＋分块表逐行显示 `台账 [seq]`；**截断路径也补写台账行**（此前只有压缩路径写） |
| **R-7（P2）** 指针化改写本地面且回读承诺不实 | **改前先逐字落盘**（`.gsa/compaction/pointerized/<tag>-<call_id>.md`），指针指向真实载体；落盘失败 ⇒ **不指针化**（不漏唯一副本）；指针文案同步改口 |
| **R-8（P2）** 先裁回放块未落 | 新增 `pointerize_replay_tool_results`（按声明参数含 `compaction/blocks` 识别回放读，**大者优先**、重读即恢复）；T1/守卫在截断后**仍越线**时按「先回放块 → 再超大结果」二级消化 |
| **R-9（P2）** 量尺不含静态开销 | `ModelFaceParams.static_overhead_tokens`（系统提示词 ＋ 工具定义；**上一轮请求实测**，首轮 0）计入阶梯/守卫/事件读数；测试缝隙 `with_model_face_static_overhead`（`with_context_scale_ladder` 自动 pin 0 以保住既有小刻度语义）。0.77 换算仍待真机复测 |
| **R-10（P2）** 两段门未提示 | 回放指针文案明写「首读若收到 `session_volume_notice` 通知信封，**再读一次**即放行」（分块表／marker／指针化行三处） |
| **R-11（P3）** 卫生 | 死字段 `target_tokens`／`min_rounds` 删除；`compact_messages`＋`CompactionStats` 删除；陈旧注释改写；`first_block` 提醒改**会话级水位**；笔误「会议会话」修正；**测试构建 10 条告警清零**（unused imports／`BlockedArchiveHost` 改为新钉子复用）；`model_face_estimate` 改**不物化**实现（`estimate_model_face_tokens`，免每轮克隆整段会话） |
| **R-12①（设计）** 天花板名存实亡 | 用户裁定：守卫 **1.10M → 700K** ＋ **T1 按越线重新武装**（H1 随之重新武装，保证每次 T1 前都先有硬提醒）；`ContextScaleState` 增 `latched`／`flags`／`rearm`；「越线但无可执行动作」⇒ **不消费闩位**（下一轮再试） |
| **R-12②（设计）** H1 打断点 | 确认＝**越线后的下一个 loop-top**（工具结果已入窗口、请求组装之前），模型看到的第一个请求即带硬提醒；320K→500K 余量 ≈180K 估算 ≈66 轮 |
| **R-12③（P2／设计）** 阶梯批量触发 | **已修**（同日落码）：一轮内**只注入最高档**——`has_truncate` ⇒ 只发截断告知块；否则仅最高档渲染。低档**水位与事件照记**（`form=suppressed_superseded_by_higher_tier`／`deferred_to_truncation_notice`）。`truncation_notice_block`／`guard_truncation_notice_block` 增 `model_face_tokens` 参数 ⇒ 告知块自带**截断后读数**（压掉同轮提醒因此无损）。新钉子：`a_single_round_surge_injects_only_the_highest_tier`、`a_truncation_round_injects_only_the_notice_with_the_post_cut_reading` |
| **R-12⑤（设计）** 回放窗口 | **已裁定：不扩窗口**（回放走两段门；机制证据与理由见审查件 §3.7 处置补录、设计 §6 补录）。本批只做文档与指针文案，**零契约改动** |
| R-12④（设计）0.77 换算 | **待真机**（不拍数）：§9 给出离线取数配方；真机首件即可复测 |

**处置批复核（2026-09-16 追加）**：orz-loop **807/0/3**（+2 钉）、orz-host 串行 **328/0/5**、
`cargo fmt --all -- --check` 干净、clippy orz-loop **50**（与处置批前持平，零新增）；
契约面零改动（无 schema／verifier／冻结镜像变更）。

**处置批读数（本机，2026-09-16）**：orz-loop `--lib` **805/0/3**（**＋R-12③ 追加处置批 2 钉 ⇒
本件最终读数 807/0/3**）、orz-host 串行 **328/0/5**、orz-assurance **229**＋fixtures 全绿、
`fmt` 干净、clippy orz-loop **50**／orz-host **12**（与基线一致）、**orz-loop 测试构建告警 0**
（处置前 10）、`runtime/tests` **361/1**（同一既有无关红灯）、门禁唯一 error＝
`orz submodule working tree is dirty`（未提交所致）。

## 9. 真机首件取数配方（R-12④ ＋ 五项真机读数；**离线可算，不需要改契约面**）

数据面＝run journal `{工作区}/.gsa/runs/<RUN-ID>/events.jsonl`（逐行 JSON；`sequence` 为全序）。
五条读数逐条给出取数字段与算法，真机首件 run 收尾时按本表取数即可，**不拍数、不做无样本推断**。

| # | 读数 | 取数（journal 字段） | 算法与判据 |
|---|---|---|---|
| 1 | **模型面工作点分位**（真实 token） | 每条 `event_type=model_output` 的 `payload.cache_hit_tokens` ＋ `payload.cache_miss_tokens` | 逐轮求和 ⇒ 该请求真实 prompt 规模；取 p50／p90／max。**判据**：p90 应落在 x（160K 估算 ≈123K 真实）→ T1（500K 估算 ≈385K 真实）区间内；超出即换算或阶梯落点需复裁 |
| 2 | **0.77 换算复测** | `event_type=mechanical_audit_update` 且 `payload.kind="context_scale"` 的行，其 `payload.payload.summary` 含 `milestone_tokens=`／`model_face_estimate_tokens=`；**按 `sequence` 取该行之后第一条 `model_output`** | 配成 (估算, 真实) 对：`真实 = hit ＋ miss`（该请求真实 prompt）⇒ `系数 = 真实 ÷ 估算`。**判据**：实测系数应落在 0.77（±10%）；若显著>0.77（真实更大）⇒ 等价于天花板被抬高，须同步压档（设计 §3.1 明文）。偏差来源三处如实标注：估算**不含**刚注入的该提醒块本身（<1% 估算）、`静态开销`取上一轮实测（首轮 0）、marker／分块表随轮变化 |
| 3 | **H1 是否被模型消费** | `payload.kind="model_compression"` 行的 `summary` | 数 `model_participated=true` 占比（＝窗口内产出语义摘要块的比例；`false` 即打断代价白付）。**判据**：0ai 基线是「提醒给了、压缩 0 次」；本项读数是 H1 打断形态（设计 §5）的取舍依据 |
| 4 | **T1 触发次数** | `payload.kind="context_scale"` 且 `summary` 含 `tier=hard_truncate` 的行（刻度行）＋ 行 `key="context_scale:hard_truncate"`（截断事实行，含 `truncated_blocks=`／`freed_tokens=`／`model_face_estimate_after=`）；上限守卫另见 `anomaly=hard_context_guard_truncated_blocks` | 计数＝本会话硬截断轮次；对照[成本重算记录](../CONTEXT_SLIDER_V8_COST_RECOMPUTATION_2026-09-16.md)的解析值（0ai 量级 ≈2 次/会话）。**判据**：显著高于 2 次 ⇒ 重写税按次计的成本结论要改 |
| 5 | **回放使用率** | `event_type=tool_started`／`tool_completed` 中 `tool=read_file` 且参数路径含 `.gsa/compaction/`（指针化回放读＝参数含 `compaction/blocks`，见 `pointerize_replay_tool_results`） | 分子＝回放读数、分母＝全部 `read_file` 调用；同时数首次 `policy_denial{code=session_volume_notice}` 与随后 `session_volume_opened` 的比例（＝模型是否读懂指针文案里的「再读一次」） |

**附带两读（同一份 journal 即可算，本批未单列）**：① 前缀缓存 hit／miss 比（`cache_hit_tokens` vs
`cache_miss_tokens`，成本重算的实测校验）；② 每轮新增真实 token（相邻两轮真实 prompt 之差 ⇒
「稳态只追加」前提的实测校验）。

**配方实核（2026-09-16，只在**现成** journal 上跑，不改任何状态）**：对 0ai 狗粮 run
`RUN-CLI-6aa999d6` 的 236 条 `model_output` 逐条取 `cache_hit ＋ cache_miss` ⇒ **每轮真实 prompt：
min 4,924／p50 81,802／p90 105,343／max 120,500**，缓存命中率 **96.70%**（与当轮账面读数一致）。
两点结论：① **配方 ①⑤ 可离线复现**，v8 首件只需重跑同一算法；② 该 run 的真实工作点**上限仅
≈120K**——即 v7「本地 500K 里程碑」时真实上传只有约 120K，而 v8 的**主滑块 x 本身就 ≈123K 真实**
⇒ v8 把工作点整体抬到 **123K–385K 真实** 区间，§9 的成本与质量结论都押在这条区间上
（故 R-12④ 的 0.77 复测与第 1 项工作点分位是真机首件必读）。配方 ② 的**估算侧字段**由单测形状
钉子钉住（`context_scale:<档>` 行含 `model_face_estimate_tokens`），端到端配对留真机首件确认。
