# ORZ-COMPACTION-REDESIGN 实施审计（2026-08-14）

- 范围：P0-D 压缩机制重设计实施（S1→S4）；权威=ADR-0010 v1.10（§3.6 / §14.10）；
  设计入口=`docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md`；用户已放行实施（2026-08-14）。
- 实施切片：S1（D2-2/D3-1 恢复前置）→ S2（工具记录机械坍缩）→ S3（五段模板摘要接线）→
  S4（审计、ADR/索引/BACKLOG/TODO/README 同步）。
- 验证：orz-loop 296 passed / 3 ignored；orz-host 208 passed / 4 ignored；
  orz-tui 178 passed；`cargo test --workspace` exit 0（含 orz-agent 570）；
  Python runtime 事件校验 164 passed；`python scripts/check_repository.py` **valid**、
  0 errors；`git diff --check` 干净。

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
| D-5 | orz-compaction 复用面=退化摘要拒绝 + MIN_SUMMARY_SEED_CHARS（选择/缩减守卫做语义等价内联） | crate 引擎为 Grok 形态（CompactionItem trait 管线），ORZ loop 自有消息模型；按复杂度治理只接守卫常量，不引入平行抽象 |
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
