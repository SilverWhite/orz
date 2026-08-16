# PLAN-FIRST 阶段 A 实施审计（2026-08-16）

> 范围：PLAN-FIRST-BLACKBOARD 阶段 A——模板去人格 + AGENTS.md 计划型包裹 +
> 首轮计划轮硬门（`docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md` §9；
> ADR-0010 §14.17）。
> 前置：P0-C 正式组件决策门（2026-08-16 用户裁决「P0-C 可转正式组件」）。
> 权威：ADR-0010 §14.17 / PLAN_FIRST_BLACKBOARD_DESIGN；本文件只登记实现
> 事实与证据，不新增设计裁决。

## 1. 验收点对照（设计 §11）

1. 首轮请求 header 工具面 = 计划轮面（含 `blackboard_read`/`plan_write`，
   无执行工具）——已实现并测试锁定（`plan_first_round_surface_is_plan_only_
   and_valid_plan_lands`：initial `RequestHeaderChange.tools ==
   ["blackboard_read", "plan_write"]`）。
2. 计划校验失败可重填一次，降级有事件留痕——已实现并测试锁定
   （`plan_first_round_refills_once_then_accepts`：refill_requested → accepted；
   `plan_first_round_invalid_twice_degrades_and_continues`：refill_requested →
   degraded，`degrade_reason=validation_failed_after_refill`）。
3. 上一步未 `done` 时下一步订单机械拒绝——阶段 C 范围（D5 步骤门），本
   阶段未实施；见 §5 边界。
4. AGENTS.md 包裹文本位于用户内容之前（渲染快照断言）——已实现并测试锁定
   （`plan_first_framework_precedes_agents_content`：`<plan_first_framework>`
   在第一个 `## From:` 之前；主/子代理同一渲染入口）。
5. 模板渲染后不含人格关键词（测试锁定）——已实现并测试锁定
   （`templates_render_without_personality_keywords`：base/subagent/
   apply-patch 三模板均不含 `released by xAI`/`friendly`/`curious`/
   `expert peers`/`aggressively` 等 12 个关键词）。
6-10. 阶段 C 验收点（console 默认面、双模式、direct 例外等）——不在本
   阶段范围，随阶段 B/C。

## 2. 实现清单

### 2.1 模板去人格（D1）

- `orz/crates/codegen/orz-agent/templates/prompt.md`：删除身份宣告
  （`released by xAI`/interactive/autonomous）与 output_efficiency 的
  人格化示例，保留机械契约。
- `orz/crates/codegen/orz-agent/templates/subagent_prompt.md`：删除
  `Grok Build subagent` 身份与 `<persona>` 模板段（roles 保留）。
- `orz/crates/codegen/orz-agent/templates/apply_patch_prompt.md`：删除
  CLI 身份、`## Personality` 段、preamble/final-message/tone 语气内容。
- `orz/crates/codegen/orz-agent/src/config.rs`：`ORCHESTRATOR_PROMPT_BODY`
  改为职责分工清单（删除 expert peers/aggressively/trust their judgment）。
- `prompt_encrypted.rs` 已用 `scripts/encrypt_templates.py` 重新生成。
- `orz-subagent-resolution/src/definition.rs`：子代理渲染不再注入
  `persona_instructions`。

### 2.2 AGENTS.md 计划型包裹（D2）

- `orz/crates/codegen/orz-agent/src/prompt/agents_md.rs`：
  `PLAN_FIRST_FRAMEWORK_BLOCK` 常量 + `render_agents_md` 固定前缀注入
  （第一个 AGENTS.md 文件之前）。

### 2.3 首轮计划轮硬门（D3）

- 新模块 `orz/crates/orz-loop/src/planning.rs`：`plan_write` 结构化计划
  机械校验（设计 §5：plan_id/goal/steps[]，每步 id/goal/actions/acceptance/
  evidence/status=pending；长度与数量上限）、一次重填 + 降级判定、
  `plan_write` v0.2 事件 payload 构造、`PlanGateState`。
- 黑板扩展（`blackboard.rs`）：`PlanStep` 结构化（goal/actions/acceptance/
  evidence）+ `PlanAction`；`rotate_to_structured_plan`（同 plan_id 修订
  复用 epoch、新 plan_id 单调递增；旧描述式 `rotate_to_plan` 保留兼容）。
- 工具面（`controller.rs`）：`plan_write` ToolDef（ReadOnly 风险类）；
  `run_host_tool_with_plan_gate` 分支：校验 → 落板（`apply_structured_plan`，
  plan epoch 归档纪律复用 F7 队列）→ `plan_write` 事件 + ToolCompleted +
  结构化结果（outcome/attempt/plan_id/plan_epoch）。
- 循环接线（`agent_loop.rs`）：会话级计划门状态（`plan_first_enabled` +
  `plan_first_session_done`，主车道且无已批准计划才触发）；首轮工具面过滤；
  计划轮内其他工具机械拒绝（`plan_round_tool_denied`）；无提交 3 轮上限
  → `plan_not_submitted` 降级放行；从结构化结果更新门状态。
- 生产接线：`orz-host/src/acp_server.rs`（每会话一次，恢复会话不重复）与
  `orz-bin/src/main.rs`（CLI run）开启开关。
- 事件契约先行：`plan_write` 加入 Rust `EventType`、`run-event-v0.2.schema.
  json` 枚举、`plan-write-event-payload-v0.2.schema.json`、Python verifier
  注册表、fixture 生成器与 envelope/payload fixtures、TUI events/bridge/
  projection。

## 3. 验证证据

- orz-loop lib：407 通过 / 0 失败（含 6 项新增 PLAN-FIRST 集成测试）。
- orz-tui lib：178 通过 / 0 失败（含 PlanWrite 投影 + ACP 生产会话计划门
  接线后的脚本更新）。
- orz-assurance lib：151 通过 / 0 失败。
- orz-agent：模板/AGENTS.md 相关 55 项通过（含 2 项新增；11 项既有
  git2 temp 路径用例在沙箱内无法运行，属环境限制、与本改动无关）。
- Python run-event 一致性：14 通过 / 0 失败（v0.2 枚举 48 项全覆盖）。
- 仓库门禁 `check_repository.py`：除「orz submodule working tree is dirty」
  （本实施未提交）外零错误；fixture/注册表计数一致。

## 4. 设计-实现符合性

- 首轮计划轮不派发执行工具：`current_tool_defs` 在计划门激活时过滤为
  `blackboard_read` + `plan_write`；派发前对任何其他工具机械拒绝并留痕。
- 一次重填 + 机械降级 + journal 留痕：`MAX_PLAN_ATTEMPTS=2`，事件
  outcome=refill_requested/degraded，与强制模板轮语义一致。
- 计划落黑板 plan epoch：结构化步骤经 `rotate_to_structured_plan` 落板，
  同一 `plan_id` 修订复用 epoch，新计划单调递增（既有不变式复用）。
- 不挂死：未提交 3 轮上限 `plan_not_submitted` 降级放行；落板失败
  （`plan_rotate_failed`）同样降级放行。
- 会话级语义：`plan_first_session_done` 保证每会话一次；恢复会话（有
  conversation sidecar）标记已完成，不重复触发。

## 5. 边界与已知项

1. **persona 配置解析保留**：`SubagentPersona`/`PersonaIOField` 类型与
   spawn `persona` 覆盖保留为外部 shell 兼容层（本仓库无 orz-shell crate）；
   模型面注入与 `<persona>` 模板段已退役。彻底删除需外部 shell 同步，
   登记为后续清理项（非本阶段阻塞）。
2. **D5 步骤状态机 / `step_not_done` 门**（验收点 3）属阶段 C 范围：本阶段
   只落结构化计划（status=pending），`done/failed(receipt_id)` 状态与订单
   门随阶段 C 实施。
3. **计划门开关**：默认关闭（测试基线兼容），生产 ACP server / CLI run
   显式开启；无环境变量覆盖。
4. **归档 claim 竞态**：首轮计划落板沿用既有 `apply_structured_plan` 的
   归档纪律（F7 队列），未额外做 `.claim-<n>` 占号（单进程内使用；跨进程
   并发由既有 epoch 单调编号 + 恢复路径兜底）。

## 6. 结论

阶段 A 三项验收（模板去人格、AGENTS.md 计划型包裹、首轮计划轮硬门）均有
实现入口与测试证据；设计-实现符合（ADR-0010 §14.17）。剩余未闭合：
PLAN-FIRST 阶段 B（注册板块=探针投影）、阶段 C（console 默认 + direct
受控降级）与 §5 边界项。

## 7. 全面审查收口（2026-08-16；ADR-0010 §14.17⑯）

阶段 A 全面审查（设计/实现/符合性三线 + 全量验证）发现并修复/登记以下
问题，全部经用户裁决或确认：

### 7.1 用户裁决

- **计划轮不消耗 tool-round 预算**：批处理前固定「计划轮身份」
  （`plan_round_active`），计划轮（含 blackboard_read/plan_write/拒绝轮）
  不执行 `tool_rounds += 1`（`agent_loop.rs` 预算块）。
- **P2-2 顺延**：compaction whitelist 的 `tool_rounds==0` 窗口随计划轮不
  计预算而顺延到计划落板后的首个执行轮；测试锁定
  （`plan_first_round_defers_whitelist_window_to_first_execution_round`）。
- **结构上限定稿**：步骤 ≤32、每步动作 16→8（对齐
  `MAX_SCRIPT_STEPS_PER_ORDER=8`）、每步证据 ≤16 条且单条 ≤500、计划整体
  序列化 ≤32K 字符。成熟参照：AutoGPT ≤5 子目标、oh-my-loop <10 子任务
  且最多 2 次重规划、joyagent 上限 40 步、编排计划工具 2-5 里程碑、
  LangChain「一步≈一次工具调用 + 有界重规划」。上限是机械兜底而非质量
  目标；未提交计划轮上限 3 保持。
- **D2 全覆盖**：AGENTS.md 为 Codex/Grok 生态契约，orz 生产路径未注入
  AGENTS.md 段——框架块提升为 orz-assurance canonical 常量
  （`plan/framework.rs`），plan_first 会话在系统提示词层无条件注入
  （主/检索子代理同一入口，不依赖 AGENTS.md 存在）；`render_agents_md`
  固定前缀保留给外部 AGENTS.md 消费者（无重复注入：orz 系统提示词不
  含 AGENTS.md 段）。

### 7.2 修复清单

1. **P1 事件契约**：plan_write ToolCompleted 收敛为成功仅 `exit_code`、
   失败 `status=error`+`error`；主车道拒绝事件（ToolStarted/ToolCompleted）
   不再携带 `target`（通用 schema target 枚举仅检索车道）；Python 新增
   `test_v02_plan_first_tool_event_shapes_validate` 锁定。
2. **P2-1 子代理面**：`subagent_tool_projection` 与
   `ToolFilter::Retrieval.write_gate` 均剔除/拒绝 `plan_write`（三重拒绝
   现在成立）。
3. **P3-4 开关收敛**：`plan_write` 声明与调用均随 `plan_first_enabled`
   收敛；关闭态/grill 拒绝 `plan_write_disabled`，不旋转黑板。
4. **P3-1 TUI**：`TuiEvent::PlanWrite` 增 `degrade_reason`，投影按
   outcome+degrade_reason 显示（plan_not_submitted/plan_rotate_failed
   不再误显「校验通过」）。
5. **P3-2 上限**：`MAX_EVIDENCE_ITEMS_PER_STEP=16`、
   `MAX_PLAN_TOTAL_CHARS=32768`、`MAX_ACTIONS_PER_STEP=8` 入机械校验并
   测试锁定。
6. **P3-3 ToolDef schema**：plan_write 参数嵌套 JSON Schema 与校验常量
   对齐（plan_id/goal/steps/actions/evidence/status）。
7. **P3-5 同轮单次**：同一计划轮第二次 plan_write 机械拒绝
   （`plan_write_already_submitted`）。
8. **测试债务**：orz-agent `child_rendered_prompt_includes_role_and_persona_sections`
   更新为断言 persona 不注入（原断言与 D1 退役直接冲突，此前导致 1 项
   真实失败）。

### 7.3 复核证据（2026-08-16 实测）

- orz-loop lib：416 通过 / 0 失败（+9：预算/whitelist 顺延/同轮单次/
  关闭态拒绝/框架开关/动作上限/证据上限/总量上限/旧归档兼容）。
- orz-tui lib：178 通过 / 0 失败；orz-assurance lib：152 通过 / 0 失败
  （+1 framework 常量测试）。
- Python run-event：一致性 15 通过（v0.2 枚举 48 项 + P1 形状测试）；
  runtime 全量 299 通过（含 5 项 PlanWrite 序列规则测试）。
- orz-agent：除 35 项沙箱 git2 临时目录权限失败外通过（persona 测试已
  更新；§3 中「11 项环境失败」的表述以此处实测为准）。
- 仓库门禁：仍仅「orz submodule working tree is dirty」（未提交）。

### 7.4 遗留登记

- `blackboard.action_write` 的 ToolCompleted 扩展字段（order_id/action/
  round/plan_epoch 等）仍不符合通用 tool-completed payload schema——S2
  遗留债务，随阶段 C console 面收敛时一并处理（不在本阶段阻塞）。
- **旧 epoch 归档兼容（已修复）**：`PlanStep` 增 `#[serde(default,
  alias="description")]`——升级前 description-only 快照可反序列化
  （description→goal，新字段取默认值，状态保留），不静默丢弃。
- **plan_write 权限分类副作用（登记为有意语义）**：ReadOnly 类仅指
  外部副作用边界——写内存黑板计划槽 + 新 plan_id 时轮换黑板（清空
  edits/tool_actions/exec）+ 机械 best-effort 归档追加（`.gsa/blackboard/
  epoch-<n>.json`），不属工作区变更。
- **会话级门 fail-open 边界（登记）**：ACP session 未找到时默认
  `plan_gate_done=true`（防御性默认，跳过门）；会话崩溃于计划轮中途且
  无已批准计划时，恢复会话按 conversation sidecar 存在即跳过首轮门——
  阶段 C 前可收紧为状态机化生命周期。
- **StepStatus 命名漂移（登记，阶段 C 对齐）**：当前枚举
  Pending/InProgress/Completed/Blocked 与设计 §2.5/§6 的
  done(receipt_id)/failed(receipt_id) 存在表述差，随步骤状态机扩展时
  统一。
- **Python verifier 序列规则（已补）**：新增 `_verify_v02_plan_write`
  ——refill_requested 仅 attempt=1 且必须被后续 plan_write 接续；
  validation_failed_after_refill 仅 attempt=2；accepted 要求
  validation.valid=true；机械降级（plan_rotate_failed/plan_not_submitted）
  要求 validation.valid=true（无结构错误）；计划修订事件不受门序列约束。
- 归档 claim 竞态、persona 配置解析保留等 §5 边界项不变。
