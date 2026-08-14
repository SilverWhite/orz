# 中立问询强制模板轮实施审计（2026-08-15）

> 状态：闭合；范围：ORZ-ORIENTATION-FORCED-TEMPLATE（BACKLOG 6c / TODO 14-18）。
> 设计权威：`docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md`；
> ADR-0010 §4.2（v1.16 正文修订）/ §14.16（实施登记）。

## 1. 结论

强制模板轮五项（ADR §4.2 修订、模板轮实现、缓解必做、契约与测试、审计同步）全部实施
闭合。2026-08-15 全面审查后完成复核修复（见 §6），修复后证据：orz-loop 333 通过
（原 323 + 新增 9 + 复核 1）、orz-assurance 151 通过、orz-tui 178 通过、
orz-bin 单测/e2e 全部通过、Python runtime tests 264 通过（新增 DC 族/outcome/
gather_evidence 交叉用例）、`scripts/check_repository.py` valid 0 错误。

## 2. 机制实现

### 2.1 触发与暂停

- 触发沿用现有规则（Orientation session-level 7 轮；DC 硬信号 2→3→4→5）。
- 主车道（Orientation + DC）在触发点注入 `[ORIENTATION v0.3]` /
  `[DIAGNOSTIC_COVERAGE v0.3]` 模板块后进入 **pending checkpoint 单槽**：下一次
  loop-top 即为无工具 checkpoint 轮（工具列表置空、跳过工具探针），模型只输出 JSON
  问询模板答案；pending 期间跳过压缩与再次触发（防止未提交计数双发）。
- 同一安全间隙两族同时到期时 Orientation 优先，DC 在下一安全间隙再触发（一次一轮）。
- 检索车道保持旧行为（fire 后继续，提交在注入点）——§14.16 边界。

### 2.2 模板与校验

- 字段：`task_position`（必填 ≤400 字）、`progress_evidence` / `blockers`
  （数组 ≤20 项、单项 ≤200 字）、`next_action`（closed enum 5 值）、
  `changed_direction`（bool）、条件字段 `missing_evidence`
  （`next_action=gather_evidence` 时必填非空）；长度上限按 trim 后值计
  （2026-08-15 复核）。DC fire 事件可选携带 `agent_role=main`（主车道恒 main），
  验证器对未携带该字段的历史 fire 兼容（2026-08-15 复核修复 P1）。
- 解析：接受 fenced ```json、裸 JSON 或含对象文本；未知字段丢弃并记
  `ignored_fields`（事件）。
- 校验：必填/枚举/长度/条件缺失面；失败给一次 `[CHECKPOINT_REFILL]` 错误反馈重填
  （attempt 1→2）；仍失败→按已填部分机械降级（`degrade_reason=
  validation_failed_after_refill`），不挂死。
- checkpoint 轮若返回 tool_calls：视为校验错误 `tool_calls_not_allowed`，不执行任何
  工具（事件序列锁定：fire 与 response 之间无 ToolStarted）。

### 2.3 计数语义

- checkpoint 轮计入已完成逻辑模型轮（`feed_round` 与普通轮同一完成点）。
- Orientation fire 提交（`commit_fire` 重置计数）与 DC 阶段推进（阈值 +1、信号清零）
  仅在模板轮 accepted 或 degraded 后发生；refill_requested 不提交。
- 降级轮按各触发族既定语义提交（Orientation 实际 fire 即重置；DC 每阶段恰一次）。

### 2.4 缓解必做

- `progress_evidence` / `missing_evidence` 与 journal 证据身份存在性交叉校验：
  身份来源 = 主车道 `EvidenceRecord.identity` + 已提交检索 ledger
  `source_id`/`source_url_or_ref`/`source_title` + DC 已检视面
  （`DebugEpisodeState.evidence_ids`）。found/missing 列表随
  `checkpoint_response.cross_check` 记录；非阻断（强制表达、不验证诚实）。
- `next_action=gather_evidence` 必填缺失面：缺失即校验错误，不静默接受。

## 3. 契约变更

- 新 v0.2 事件 `checkpoint_response`：`runtime/checkpoint-response-event-payload-v0.2
  .schema.json`；`runtime/run-event-v0.2.schema.json` 枚举 45→46。
- Python 验证器 `assurance/run_event_journal_validation.py`：注册表 + 新增
  `_verify_v02_checkpoint_responses` 交叉校验（fire↔response 引用/inquiry_kind/
  agent_role 一致〔fire 携带时比对〕、attempt 1-2 顺序、refill 后必有 follow-up、
  degraded 必有原因、accepted 后无后续 attempt）+ 2026-08-15 复核新增
  outcome↔validation 一致性（accepted⇒valid+非空响应；refill/degraded⇒invalid）
  与 gather_evidence 条件面复核。
- fixture 生成器 `scripts/generate_run_event_fixtures.py` 增
  `checkpoint_response` good/bad 样例并重新生成；conformance 计数 45→46
  （v0.2 envelope 47 正例含 chained-run-finished）。
- Rust `EventType::CheckpointResponse`；TUI `TuiEvent::CheckpointResponse`
  （events/bridge/projection）；`is_injected_block_text` 注册
  `[CHECKPOINT_REFILL`。
- `ORIENTATION_BLOCK` v0.2→v0.3（模板 JSON 单一来源
  `TEMPLATE_ANSWER_INSTRUCTIONS`，DC 块同源）；ACAF signer 模板常量自动跟随。

## 4. 测试

新增 orz-loop 用例（+9，332 通过）：

| 用例 | 覆盖 |
|---|---|
| `orientation_forced_template_pauses_then_accepts_and_resumes` | 触发→暂停→填表→恢复；fire↔response 间无工具事件；accepted 后计数提交 |
| `orientation_forced_template_refills_once_then_accepts` | 一次错误反馈重填；attempt 1→2；失败不提交 |
| `orientation_forced_template_degrades_after_two_invalid_answers` | 两次失败→降级（原因字段）；降级仍提交；不挂死 |
| `checkpoint_round_tool_call_is_refused_without_execution` | tool_calls 校验错误、零执行、重填恢复 |
| `dc_forced_template_degrade_commits_stage_and_fires_next` | DC 降级仍推进阶段（2→3），下次 fire 需下一阈值 |
| 既有 DC 三用例更新 | 阈值推进/每阶段一次/通过重置，均带模板轮 |
| `subagent_lane_feeds_and_fires_orientation` 扩展 | 检索车道无 `checkpoint_response`（主车道隔离） |

2026-08-15 复核新增（oracle 为上述 Python 验证器与 Rust 单测）：

| 用例 | 覆盖 |
|---|---|
| Python DC 族 fire↔response（旧 fire 无 agent_role / 新 fire 携带 main / 角色不匹配） | 修复 P1 后的 DC 契约闭环 |
| Python outcome↔validation 一致性（accepted/refill/degraded） | P2-1 验证器交叉 |
| Python gather_evidence 条件面（missing_evidence 非空 + flag=true） | P2-2 验证器交叉 |
| Rust `length_limits_apply_to_trimmed_values` | P3 trim 长度口径 |

## 5. 边界与未做事项

- 不做语义诚实验证；交叉校验非阻断。
- 不做会话级强制（仅触发时暂停）；不新增工具面（无 submit 工具）。
- 检索车道与 legacy Python orientation 轨道不改（`assurance/orientation_runtime_guard.py`
  仍为历史 conformance 轨道，非生产路径）。
- 既有真实 journal fixtures 不含 `checkpoint_response`（属正常——响应事件仅在强制
  模板轮发生时写入；验证器对缺省情况无要求）。
- orz-agent（codegen）35 项测试因沙箱对 `%TEMP%/.git` 路径访问拒绝失败，与本次
  改动无关（未触碰 orz-agent）；clippy 全目标受 orz-tools-api 构建脚本缺 `protoc`
  环境限制，本次改动 crate（orz-loop/assurance/tui/bin）无新增告警。

## 6. 二次全面审查修复记录（2026-08-15 审查后）

全面审查（设计/实现/契约三维）发现并已修复：

- **P1**：DC 族 `checkpoint_response` 的 `agent_role=main` 与 DC fire（原无
  `agent_role` 字段）在 Python 验证器中恒误报不匹配。修复：DC fire payload/Schema
  增加可选 `agent_role=main`（fixtures/生成器同步），验证器仅在 fire 携带该字段时
  比对，补 DC 族交叉用例。
- **P2-1**：验证器新增 outcome↔validation 一致性（accepted⇒valid=true 且响应非空；
  refill_requested/degraded⇒valid=false）。
- **P2-2**：验证器新增 gather_evidence 条件面复核（response.missing_evidence 非空 +
  cross_check 标志为 true）。
- **P3**：`checkpoint_response.agent_role` Schema 收紧为主车道 `main`（与设计范围
  一致）；Rust 长度校验按 trim 后值；conformance v0.2 计数测试更名
  `test_v02_all_46_event_types_covered` 并补 45→46 说明；设计/ADR 补
  `missing_evidence` 交叉校验与 pending 中途终止的孤儿 fire 语义说明。
