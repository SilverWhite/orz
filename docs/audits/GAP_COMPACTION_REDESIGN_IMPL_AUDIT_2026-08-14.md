# ORZ-COMPACTION-REDESIGN 实施审计（2026-08-14）

- 范围：P0-D 压缩机制重设计实施（S1→S5）；权威=ADR-0010 v1.10 + v1.14（§3.6 / §14.10 / §14.14）；
  设计入口=`docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md`；用户已放行实施（2026-08-14）。
- 实施切片：S1（D2-2/D3-1 恢复前置）→ S2（工具记录机械坍缩）→ S3（五段模板摘要接线）→
  S4（审计、ADR/索引/BACKLOG/TODO/README 同步）→ S5（审查修复，见 §7）。
- 验证：orz-loop 305 passed / 3 ignored；orz-host 208 passed / 4 ignored；
  orz-tui 178 passed；`cargo check -p orz-host -p orz-tui -p orz-bin` exit 0；
  Python runtime 事件校验 171 passed（conformance + journal validation）；
  `python scripts/check_repository.py` **valid**、0 errors；`git diff --check` 干净。

## 1. S1 — 恢复路径前置（D2-2 / D3-1）

### D2-2 恢复超窗预估算截断

- 落点：`run_turn_inner` 消息种子后、首请求前（`controller.rs`）——仅对
  `conversation.is_some()`（恢复/累积会话）生效；grill 排除（互斥断言既有）。
- 触发：`estimate_messages_tokens`（chars/2，保守 CJK 口径）> `recovery_trigger_tokens`
  （默认 200K = min(224K 有效输入预算, 200K 兜底线)）。
- 动作：复用 `compact_messages` 整轮丢弃到 `recovery_target_tokens`（默认 160K）；
  preamble/白名单/最近轮保留；插入恢复 marker（`[前文上下文已压缩 v0.1-恢复]`，
  复用 CONTEXT_COMPRESSED_PREFIX，D3-1 可保留）；完整侧车序列化到运行目录
  `recovery-conversation-full.json` 作为审计副本；journal `context_recovery_truncated`
  事件（before/after 估算、丢弃轮数、审计路径）。
- 事件契约：`EventType::ContextRecoveryTruncated`（v0.2 枚举 43→44）、payload schema
  `context-recovery-truncated-event-payload-v0.2.schema.json`、Python verifier 映射 +
  `_verify_v02_recovery_truncation` 交叉规则（必须位于首个 model_request 之前且至少
  丢弃 1 轮）、fixtures（payload 正/反 + envelope）由生成器重建。

### D3-1 marker/白名单恢复保留

- 主车道写回过滤与子代理 activation snapshot 过滤统一改为：User 角色注入块中
  **marker（`[前文上下文已压缩`）与白名单块（`[压缩白名单`）保留**，其余机械注入块
  仍过滤（`prompt::is_restore_retained_block`）。
- 恢复后首请求可见 marker 与白名单（测试 `conversation_writeback_retains_marker_and_whitelist`
  + `recovery_conversation_over_window_truncates_before_first_request`）。

## 2. S2 — 工具调用记录机械坍缩

- 新模块 `orz-loop/src/action_ledger.rs`：每模型请求的**模型可见视图**将已完成的旧轮
  坍缩为确定性动作台账块（`[动作台账 v0.1]`，每工具调用一行：轮次、工具名、目标
  （path/file/url/target/document_id/directory/command 机械提取）、结果指针
  （sha256 摘要）、非空最终回复原文）；`messages` 本体不变（journal/侧车保留完整记录）。
- 纪律：整轮配对（assistant 声明 + tool 结果不拆）；缺结果的轮不坍缩（保留有界尾部）；
  最近 `recent_tail_rounds`（默认 2）轮始终原文保留；零模型调用、无冷却。
- 接线：`agent_loop.rs` 的 `run_round` 请求由 `build_collapsed_request` 构造；
  三 Agent（主 + 内外检索 + grill）共用同一循环同一策略。
- 事件/schema 零变更（输入侧透明变换）。

## 3. S3 — 五段模板摘要接线

- 新模块 `orz-loop/src/summary.rs`：
  - 五槽（目的/计划/变动文件路径 机械填充自黑板；注意事项/后续衔接 模型生成）；
    字符上限 3/3/5/3/3K=17K。
  - 机械校验：槽齐全、每槽 ≤ 上限、合计 ≤17K；退化拒绝复用 orz-compaction
    `is_degenerate_summary`（MIN_SUMMARY_SEED_CHARS=500）；重做 ≤3 次。
  - 终止态：仅机械段 + marker `summary_incomplete` + 扩大最近尾 +1；fallback 路径
    同时整轮机械截断（D2-2 紧急语义，不允许停在 200K+）。
- 触发（loop-top 任意安全间隙，推翻"仅最终答案间隙"）：实测 prompt tokens >
  `trigger_tokens`（160K）且冷却 ≥`min_rounds`（3）；或 >`safety_tokens`（200K）无视
  冷却；缩减守卫 `min_compactable` 5K / `max_reduction_ratio` 0.6。
- 摘要调用：会话模型（DeepSeek V4 gateway）纯 chat 调用、无工具、不进探针/压缩回路、
  不消耗工具轮预算、不计数 orientation 轮。
- 存档与 marker：`{cwd}/.gsa/compaction/compaction-<run>-<seq>.md`（含 digest 绑定、
  derived_unverified 标记）；滚动单 marker（旧 marker 随 retain 移除）；marker 内容=
  摘要 ID/被压轮次/存档路径/digest/五槽内容/回查分区。
- 事件：`context_compressed` v0.2 payload（mode=template_summary、reason=rhythm/fallback、
  summary_id/digest/path、summary_incomplete、retained_rounds；旧 v0.1 文件保留 replay）；
  Python verifier 映射 + `_verify_v02_context_compressed` 交叉规则；fixtures 重建。
- retention：`.gsa/compaction` 纳入 7 天清扫（`PruneReport.removed_compaction_archives` +
  测试）。
- TUI：新增 `ContextRecoveryTruncated` 投影；`context_compressed` 投影沿用既有字段。

## 4. 决策与登记（实施时定案）

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | S2 坍缩只作用于**请求视图**，`messages` 本体不动 | §3 审计双轨：完整记录保留在 journal/侧车/P4；模型上下文只保留台账行；避免侧车丢失工具全文 |
| D-2 | 摘要成功时**真实截断** conversation（preamble+marker+最近尾）并落档 | §4 目标=17K 摘要 + 白名单 + 最近尾；journal 与 `.gsa/compaction` 存档承担审计 |
| D-3 | 恢复截断事件独立为 `context_recovery_truncated`（不复用 context_compressed 文案字段） | ADR §5.1 明确类型；D2-2 是独立机制事件 |
| D-4 | 路径槽 Top-N=40、按黑板编辑插入序（时间序） | 设计 §10 未决项定案；全量路径仍在 blackboard_read edits 可查 |
| D-5 | orz-compaction 复用面=退化摘要拒绝 + MIN_SUMMARY_SEED_CHARS（选择/缩减守卫做语义等价内联）；**S5 修订（v1.14）**：退化守卫改为 ORZ 自定 300 等效字符门（CJK 一字折算 2），orz-loop 不再依赖 orz-compaction | crate 引擎为 Grok 形态（CompactionItem trait 管线），ORZ loop 自有消息模型；500 字符门为英文向标定，中文场景需独立折算 |
| D-6 | 摘要调用不计工具轮、不计 orientation 轮；三 Agent 同构触发 | 同构约束（§3.1）；activation 预算交互复核无新计数面 |

## 5. 边界与登记

- `target_tokens`（旧 90K）仅保留给纯 `compact_messages` 单元面；loop 不再使用。
- 检索子代理的压缩触发随各 lane 自身实测用量自然发生（与激活预算互不计数）；
  摘要失败不消耗检索候选预算。
- 摘要存档按 7 天 retention 清扫；滚动 marker 为唯一常驻指针。
- 恢复截断审计副本位于运行目录（run 级证据），不污染会话侧车。
- 旧 A6 语义测试（仅最终答案间隙、20 轮冷却、250K 兜底）已按 v1.10 重写/替换。

## 6. 与相邻文档衔接

- `GAP_CONVERSATION_RESTORE_IMPL_AUDIT_2026-08-10.md` §4 边界 D2-2/D3-1 —— **本审计闭合**。
- ADR-0010 v1.10 §14.10 —— 补实施闭环注记。
- `docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md` —— 状态转 implemented，§9/§10 更新。
- `CLI_PROJECT_INDEX.md` —— FUS-COMPACTION-REDESIGN 转 implemented。
- `TODO.md` / `docs/BACKLOG_AND_PRIORITIES.md` —— P0-D 全部勾选/关闭。

## 7. S5 审查修复实施（2026-08-14，ADR-0010 v1.14）

- 范围：P0-D 全面检查发现的差距按用户逐项裁决修复；权威=ADR-0010 v1.14 §14.14。
- ① 守卫失败重试与强制压缩：`agent_loop.rs` 新增 `GUARD_RETRY_LIMIT`=3、
  `CompactDecision`（NoOp/GuardBlocked/Executed）与共享压缩流 `run_template_compact`；
  loop-top 守卫不满足时 `GuardBlocked` 计数，跨触发轮重试（不打断内容、不机械截断），
  第 3 次仍失败即 force=true 执行一轮压缩，事件带 `guard_failed=true`、marker 附
  "机制失败：缩减守卫连续不满足，已强制压缩，需处理"；摘要 LLM 失败路径的 fallback
  机械截断保留（终止态语义不变）。测试
  `context_compact_guard_failure_retries_then_forces`。
- ② 会话结束压缩治本：`run_turn_inner` 与检索子代理 dispatcher 在 stagnation
  Continue、terminal 事件前（sidecar 写回前）调用同一压缩流（`reason=session_end`、
  force=true、不消耗工具轮/orientation 轮）；`ContextCompactConfig` 新增
  `session_end_trigger_tokens`（默认 160K，全量消息估算触发）；marker 随 D3-1
  过滤写回 sidecar 固定存档；D2-2 恢复预检保留为旧侧车兜底。测试
  `session_end_compact_pins_marker_into_sidecar_and_rolls_edits`。
- ③ 存档写失败显式重试：`summary.rs` 新增 `ARCHIVE_WRITE_MAX_ATTEMPTS`=3 与
  `write_archive_retry`；仍失败时事件带 `archive_write_failed=true`、marker 附
  "存档写入失败：摘要未落盘，需处理"，不再静默忽略。测试
  `summary_archive_write_failure_is_reported` + `archive_write_retry_persists_and_reports_failure`。
- ④ 退化守卫：废止 orz-compaction 500 字符门，ORZ 自定 `SUMMARY_MIN_EFFECTIVE_CHARS`=300，
  CJK 表意字一字折算 2 等效字符（150 汉字达标）；orz-loop 移除对 orz-compaction 的依赖。
  测试 `degenerate_guard_english_threshold` / `degenerate_guard_chinese_counts_double`。
- ⑤ 黑板窗口滚动：压缩成功后 `blackboard.edits.clear()`（擦干净），路径槽天然为本窗口
  增量；路径槽按 Top-40 条 + 5K 字符双上限，溢出指针指向本次摘要存档。测试
  `render_paths_caps_at_top_40_with_archive_pointer`。
- ⑥ 冷却与超时：`min_rounds` 默认 3→2（模型轮口径确认）；摘要调用加 120s 专用超时
  （`SUMMARY_CALL_TIMEOUT`）；marker token 估算 2K→9K（对齐 17K 字符上限）。测试
  `context_compact_defaults_follow_v1_14_review`。
- ⑦ 事件契约：`context_compressed` v0.2 reason 增 `session_end`、新增可选
  `guard_failed`/`archive_write_failed`（旧 payload 可 replay）；verifier 交叉规则
  （guard_failed 仅 rhythm/fallback、archive_write_failed 仅完整摘要）+ 合成 journal
  测试 + fixtures 正样例（session-end / guard-failed / archive-write-failed）；
  `context-recovery-truncated` schema `$id` 修正为 v0.2。
- 决策与登记：压缩流程重构为共享函数（loop-top 与 session_end 同构）；`LoopOutcome`
  增加 `rounds_since_compact` 以便 session_end 事件诚实上报冷却；终止态事件的
  `target_tokens` 改为实际保留估算（原固定 160K 在 rhythm 终止态下失真）。
- 已知生成器漂移（非本次引入，登记为卫生观察）：`generate_run_event_fixtures.py`
  的 `FIXTURES_README_V02` 与 `citation_validation` 负样例/信封时间戳未与提交树
  完全同步（P0-B step 5 的手工修订未回写生成器）；本次重生成后已手动还原无关文件，
  未回改生成器模板以避免噪音；建议后续登记 P3 卫生项统一对齐。

## 8. 二次复查处理登记（2026-08-14，用户要求处理复查全部问题）

- 范围：P0-D 二次全面复查（设计合理性/实现合理性/符合性）发现问题的全部处理；
  权威=ADR-0010 v1.14 + §14.14 条目 2；不新增设计语义，只做口径对齐与卫生修复。
- ① 设计投影 §7 复用边界对齐：原"直接复用 orz-compaction（select.rs、缩减守卫、
  退化摘要拒绝 MIN_SUMMARY_SEED_CHARS=500）"改为"语义等价内联"——orz-loop 仅依赖
  orz-assurance/orz-secrets（Cargo.toml 事实），退化守卫为 ORZ 自定 300 等效字符门；
  历史"必须适配/不引入"约束保留。设计投影 §7 与 ADR §14.14 条目 2 同步。
- ② 会话结束压缩触发阈值补写：ADR §3.6、§14.14 ①-② 与设计投影 §6 明确
  "全量消息估算 > `session_end_trigger_tokens`（默认 160K）才压缩"；实现不变
  （`ContextCompactConfig::default()` 已如此，见 §7-②）。
- ③ 冷却残留修正：`agent_loop.rs` 循环注释与
  `context-compressed-event-payload-v0.2.schema.json` 中
  `rounds_since_last_compaction` 描述的"≥3-round cooldown"改为 2 轮（v1.14 口径）。
- ④ 终止态 marker 占位 digest 修正：`summary.rs` 终止态不再写 64 位 "0" 占位，
  `build_summary_marker` 对空 digest 输出"摘要 digest: （未生成——摘要重试失败）"，
  与事件 `summary_digest=null` 一致；测试 `incomplete_marker_flags_state` 更新。
- ⑤ fixtures 生成器回写对齐（P0-B step 5 手工修订回写）：`FIXTURES_README_V02`
  与提交树 README 逐字对齐（44 events、P0-D S1/S3/S5 段落、14 个 journals 表、
  2 空格缩进）；`PAYLOAD_BAD_V02["citation_validation"]` 移除 `message_block`；
  新增 `V02_ENVELOPE_TIMESTAMP_OVERRIDES`（citation-validation 信封时间戳
  2026-08-14）；新增 `_compact_citation_reason_codes` 后处理保持三个
  citation-validation 文件的紧凑单行数组格式；`write_json`/README 写回统一
  LF 行尾（对齐 .gitattributes eol=lf）。验证=提权重生成后
  `git diff --name-only` 为 0、`git hash-object` 与 HEAD blob 逐文件一致。
- ⑥ 恢复预检估算校准（chars/2 对中文可能低估）：登记为 ORZ-SESSION-CONTEXT-MONITOR
  （P1 6d）实施前置校准项，BACKLOG/TODO 同步，不单独占 P0-D 切片。
- 验证：orz-loop 305 passed / 3 ignored；orz-host 208 / 4 ignored；orz-tui 178；
  Python runtime 事件校验 157 passed、压缩相关 41 passed；`check_repository.py`
  valid、0 errors；`git diff --check` 干净。

## 9. v1.15 后续修订注记（2026-08-14）

- 本审计 §7 ⑤「黑板窗口滚动（压缩成功后 `blackboard.edits.clear()`）」已由用户裁决废止
  （设计层面）：黑板生命周期改按 plan epoch 轮换，压缩不再清黑板；实现改造未开始，
  登记于 [BACKLOG 6e](../BACKLOG_AND_PRIORITIES.md) / [TODO](../../TODO.md)
  （ORZ-BLACKBOARD-PLAN-EPOCH）。
- 本文件保持为 v1.14 实施记录；新机制实施审计另行出具。
- 入口：[黑板 plan epoch 设计](../BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md) /
  [ADR-0010 §14.15](../../adr/ADR-0010-vol-14-addenda-index.md)。
