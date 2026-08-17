# 计划-执行分离与黑板化指挥：模型面重构设计（2026-08-15）

> 状态：**定案**（2026-08-15 用户裁决；按审查建议定稿，进入实施路由）。
> 2026-08-15 用户裁决：放弃「一条路走到黑 / 直接执行面永久移除」，定案双模式
> （console 默认 + direct 受控降级，见 §7）；同日定稿复核项——计划门约束 console
> 订单、direct 为有记录的例外（`console_step_done` 证据门）、3 连败助理层故障面
> 计数、无工具询问轮、只读豁免清单、术语与 Profile/Bundle 对齐。
> 权威登记：ADR-0010 §14.17（v1.17 补写登记，2026-08-15）。
> 说明：本文件为当前设计要求；实施阶段见 §9，实施勾选见 TODO P0-C，
> 优先级与决策门见 BACKLOG P0-C。
> 关联：CLASSICAL-EXEC-ASSISTANT v0.6（本设计为其收编）；FUS-ORIENTATION-FORCED-TEMPLATE
> （硬门/无工具轮先例）；FUS-BLACKBOARD-PLAN-EPOCH（计划落板与轮换）；FUS-TOOL-PROBE
> （注册板块=探针投影唯一事实源）。
> 背景：DeepSeek V4 Pro 对 agent scaffold 强敏感（社区实测：minimal 99/96 vs standard 91；
> 首请求 persona + 工具 schema 面决定整条会话轨迹；spec/react 之间存在不稳定混合带）。
> orz 选择计划型确定性锚定：计划与执行分离，主模型指挥助理层并以只读方式核查。

## 1. 目标

- 让主模型稳定落在计划型（spec）行为带，避免 spec/react 混合中间态。
- 消除“计划阶段不知道工具栏”导致的工具幻觉：首轮只暴露黑板，黑板注册板块是模型可见
  工具面的唯一外部准确信息源。
- 执行全部收口到助理层单一发放出口；模型只指挥与核查，不直接产生执行。
- 用机械状态机兜底“惯性幻觉”：分步计划未核查完成，下一步订单无法下发。

## 2. 设计决策

### 2.1 D1 模板去人格（除机械契约外全删）

范围：

- 主模板 `templates/prompt.md`；
- 子代理模板 `templates/subagent_prompt.md`；
- apply-patch 模板 `templates/apply_patch_prompt.md`；
- `ORCHESTRATOR_PROMPT_BODY`（`orz-agent/src/config.rs`）。

删除内容（人格/身份/语气）：

- 身份宣告：`You are Grok released by xAI`、`You are a Grok Build subagent`、
  `interactive CLI tool / autonomous agent` 等；
- 语气与人格：`expert peers, not junior helpers`、`Use them aggressively and liberally`、
  `trust their judgment`、`light, friendly, curious`、`excellent technical blog post`、
  apply-patch 模板的 `## Personality` 段与 preamble 语气示例；
- orchestrator body 中的情绪化表达，只保留职责分工清单。

保留内容（机械契约）：

- `<user_query>` 封装、action_safety、tool_calling、background_tasks；
- formatting 的机械渲染规则（markdown、文件引用、代码块格式）；
- project_instructions_spec、user_info、user_guide（功能指针，非人格）；
- orchestrator 职责分工（规划/协调/审阅、委托编辑/构建/深探索、briefing 要求、
  验收标准、并行、anti-patterns 中“不自行实现/不越权”类机械约束）。

persona 机制：

- persona 默认不注入（维持现状）；
- 内置 personas 目录、`SubagentPersona` 解析与子代理 `<persona>` 模板段退役；
- roles（能力/工具契约）保留，与 persona 分离。
- 保留中性角色声明（如 `You are an AI coding agent operating in a workspace`）
  ——无品牌、无人格语气；「身份宣告全删」指删除品牌/人格化宣告，不含中性
  机械角色句（2026-08-16 审查收口明确）。

实施注意：模板经 XOR 加密（`prompt_encrypted.rs`），修改模板后必须同步重生成
（`scripts/encrypt_templates.py`），并保留“模板不含人格关键词”的渲染测试。

### 2.2 D2 AGENTS.md 计划型机械包裹

- 计划型执行框架块（草案见 §7）为 orz-assurance canonical 常量：
  - orz 生产路径：plan_first 会话在系统提示词层**无条件注入**（主/检索子代理
    同一入口，不依赖 AGENTS.md 是否存在；2026-08-16 审查收口）；
  - 外部 AGENTS.md 消费者：仍作为 `render_agents_md` 固定前缀注入
    （用户内容之前）；
- 唯一机制：不做规范模板、不做 schema 校验、不要求项目文件改造；
- 对主代理与子代理统一生效（子代理同样注入 AGENTS.md）；
- 本框架只约束执行风格，不覆盖项目文件中的事实与约束（构建命令、代码规范、安全红线）。

### 2.3 D3 首轮计划轮（硬门）

- 触发：主车道 session 首轮。首轮不派发任何执行工具。
- 首轮模型面：
  - 读：`blackboard_read`（workboard/plan 分区 + actions 注册板块分区）；
  - 写：`plan_write`（唯一写面，写结构化分步计划）；
  - 禁止：执行、变更、shell、子代理 spawn、检索、`action_write`。
- 注册板块：由机械探针面派生（探针完整集 ∩ Profile/Bundle 加载集，与
  CLASSICAL-EXEC §8 用语一致），是模型可见工具面的唯一事实源；
  S2 静态基础集仅为接线前中间态，S3 接线后注册板块与执行面天然一致，无需额外相等约束。
- 校验：计划必须满足 §5 结构；一次错误反馈重填，仍失败机械降级并留痕（参照强制模板轮），
  不挂死。
- 通过后：controller 将计划落黑板（plan epoch），进入第二轮。

### 2.4 D4 助理层全承接（console 默认；direct 受控降级——双模式）

- 模型写面：`plan_write` + `action_write`（单动作或 PTC 线性脚本订单）。
- 模型读面：`blackboard_read`（workboard/计划、actions 注册板块、结果栏）+
  `assistant.trace`（只读）+ 工作区只读工具（read/list/grep 类）——用于核查助理层完成情况。
- 禁止面（console 默认态）：直接执行、文件变更、shell、子代理 spawn、检索工具；
  子代理与检索全部通过订单下发。
- 双模式定案（2026-08-15 用户裁决）：默认 console（§4 面）；direct 为受控降级——
  助理层连续 3 次故障面失败（§7.2 口径）后机械询问，模型选择后切换并全程留痕（§7）。
  direct 不改变权限，只恢复工作工具可见性。
- 前置条件（理想态）：
  - 工具栏刷新绑定黑板模型栏：注册板块由黑板状态派生，替代 loop-top 静态刷新；
  - 助理层实战验证稳定可靠后，再全量切 console 默认（阶段 C）；当前阶段主车道保留既有探针工具面，
    首轮计划轮照常生效。

### 2.5 D5 分步计划硬契约

- 计划 = 有序步骤数组；每步含：id、goal、actions（对应订单/动作）、acceptance（验收标准）、
  evidence（证据要求）。
- 步骤状态机：`pending → in_progress → done(receipt_id) | failed(receipt_id)`。
- 机械门：下一步订单发放要求上一步存在 `done` 状态与 receipt_id，否则拒绝
  （`step_not_done` 类明确错误码）。
- 提示词显式告知主模型：每一步先核查上一步完成（receipt/证据/产物），再下发下一步；
  禁止按计划惯性推进。

### 2.6 D6 反馈回路与核查面完备性

- 回路：订单 → 机械发放（注册表/契约/ACAF/策略门）→ 执行 → 结果栏 receipt + trace_id →
  模型读结果栏/`assistant.trace`/工作区只读核查 → 通过则推进步骤，失败则基于 fail-closed
  信封（step/code/message/upstream/trace_id）诊断后重试或修订计划。
- 核查面完备性约束：动作契约响应 schema 与 trace 事件必须覆盖助理层全部副作用出口；
  存在无法核查的出口即为设计缺陷。

## 3. 首轮计划轮机制

- 触发/暂停：run 起始进入计划轮；pending 单槽；计划轮不派发执行工具。
- 校验与降级：一次重填 + 机械降级 + journal 留痕（复用强制模板轮语义）。
- 计数：计划轮计入已完成逻辑模型轮。

## 4. 模型面定稿

| 面 | 工具 | 说明 |
|---|---|---|
| 写面 | `plan_write` | 分步计划（计划轮/计划修订时使用） |
| 写面 | `action_write` | 单动作或脚本订单（阶段 C 全量开放；首轮禁止） |
| 读面 | `blackboard_read` | workboard/plan、actions 注册板块、结果栏；有界渲染 |
| 读面 | `assistant.trace` | 只读执行日志，按 trace_id 取回 |
| 读面 | 工作区只读工具 | read/list/grep 类，核查产物 |
| 禁止面 | 执行/变更/shell/子代理/检索 | 全部经订单下发 |

> 本表为 **console 默认面**。direct 受控降级时恢复探针过滤后的工作工具投影（§7）。
> 只读豁免：`read_file` / `list_dir` / `grep` 等只读核查工具保留；执行/变更/shell/
> 检索/子代理等执行与发送面隐藏。

读面机械契约（2026-08-17 用户裁决，大文件读取契约）：工作区只读读取超过粗门
（默认 16KB、可配 8–32KB）的文件返回读取句柄信封（path / size / encoding /
content_sha256 / 可用范围 / 有界预览 ≤2–4KB / truncated / offset 续读指针），
不返回全文；小文件保持全文一次返回。模型用 `read_file(offset)` / `grep` 做结构化
续读，语义适配留在模型（助理层只提供机械原语）。黑板/结果栏只放指针
（path/document_id/size/digest/offset），不放内容本体；内容留在盘上或内容寻址
证据区；维持「不新增自由随记区」约束（ADR-0010 §3.6）。详见
[`CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md`](CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md)
§11 / ADR-0010 §14.22（v1.22）。**2026-08-17 实施闭合**（GrokBuild `read_file`
文本路径信封 + 有界预览 + offset 续读；实施审计
`docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md`）。
**2026-08-17 全面检查修复**：信封 offset 空窗口/越界语义收口（past-EOF 无续读
指针、范围内空窗口从请求行续读、最后一行行内截断 offset=None），
ADR-0010 §14.22 项 4 登记。

grep 搜索范围契约（2026-08-17 用户复核定案）：grep 返回结构化搜索信封
（resolved root / files_searched / files_skipped / match_count / truncated），
「搜索 0 文件」与「真无匹配」机械分型（searched=0 显式报范围空与过滤类别，不叫
"No matches found"）；「范围/截断必须机械报告」为读/搜/列三族统一契约（与读取
信封同构，list_dir 补 ignored/truncated 计数）。详见
[`CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md`](CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md)
§12 / ADR-0010 §14.23（v1.23）。

## 5. 分步计划 Schema（草案）

```json
{
  "plan_id": "plan-<uuid>",
  "goal": "任务目标",
  "steps": [
    {
      "id": "s1",
      "goal": "明确任务与代码结构",
      "actions": [
        {"step_id": "s1", "do": "workspace.read_file", "with": {"path": "..."}}
      ],
      "acceptance": "已确认目标与相关文件",
      "evidence": ["path:line 或 receipt_id"],
      "status": "pending"
    }
  ]
}
```

> 订单（`action_write`）携带 `step_id` 绑定计划步骤：ActionOrder 增 `step_id`
> （Schema 先行扩展，事件/verifier/fixtures 同步后再接线 producer）。

结构上限（2026-08-16 审查收口定稿；成熟参照：AutoGPT ≤5 子目标、oh-my-loop
<10 子任务且最多 2 次重规划、joyagent 配置上限 40 步、编排计划工具 2-5 里程碑、
LangChain「一步≈一次工具调用 + 有界重规划」）——上限是机械兜底而非质量目标：

- 步骤数 ≤ 32；每步动作数 ≤ 8（对齐 P0-C `MAX_SCRIPT_STEPS_PER_ORDER=8`）；
- 每步证据条数 ≤ 16、单条 ≤ 500 字符；
- 计划整体序列化 ≤ 32K 字符（`MAX_PLAN_TOTAL_CHARS`，与轮内注入预算同一量纲）；
- 未提交计划轮上限 3（`plan_not_submitted` 降级，不挂死）。

实施前经 Schema 先行扩展（事件/verifier/fixtures 同步），再接线 producer。

## 6. 步骤状态机与防惯性幻觉

- 迁移：`pending → in_progress`（订单发放时）→ `done(receipt_id)` / `failed(receipt_id)`。
- 机械门：下一步订单发放要求上一步 `done` 且 receipt_id 非空；不满足拒绝并返回明确错误。
- 防惯性幻觉：模型无法在未核查的情况下按计划惯性推进；执行结果以 receipt + trace_id 为准，
  不以计划文本中的预期为准。
- 事件面（P0-E 第 4 项，2026-08-17，ADR-0010 §14.21 项 3）：所有发放前拒绝统一写 v0.2
  `console_order_rejected`（order_id / step / phase / code / reason / round / plan_epoch /
  run_id）——pre_issue = order_stale / step_not_done / budget_insufficient；
  issue = 注册表 / 契约 / 目标 / ACAF / 策略 / 模式门（ACAF/模式/权限归一化
  step=policy / code=policy_denied）；结果栏 receipt 保留为人类可读视图，执行失败
  （execute/verify）仍只经 tool_started/tool_completed 留痕。
- 模式范围：本机械门约束 **console 订单发放**；direct 模式为有记录的例外（§7.5），
  不借降级绕过权限/ACAF/模式门。

## 7. 双模式机制（2026-08-15 用户裁决）

> 定案：放弃「直接执行面永久移除」；模型面采用双模式——console（默认，§4 面）与
> direct（受控降级）。direct 不是默认路径，也不是提权：只恢复工作工具可见性，
> 权限桥/ACAF/模式门/IPG/预算/探针全部照旧。

### 7.1 状态机

- console（每 run 起始）→ direct：唯一入口=显式询问后模型选择 switch。
- direct → console：模型调用 `console_return_to_console`（单向返回，记录事件）；
  或 run 结束自动复位。
- 检索子代理车道不受影响（无操作台，投影恢复逻辑不变）。
- 模式为 run 级状态：plan epoch 轮换/黑板旋转不清除模式；gate_log 中的 transition
  记录随黑板保留（模型可读）。

### 7.2 触发与计数（机械定义）

- 递增：receipt `ok=false` 且属于**助理层故障面**：
  - `step=verify`（响应验证失败——助理层/契约问题）；
  - `step=execute` 且 `upstream.exit_code=None` 或 host 调用返回错误（机械故障，
    无业务结果）。
- 不递增：`step=execute` 且 `exit_code=Some(非零)`（目标业务失败：测试失败、编辑不匹配、
  文件不存在等，不是「助理层不可用」）；`policy`（合法拒绝，走既有 denial breaker）；
  `protocol/registry/contract`（模型订单写错）；`order_stale`；`step_not_done`。
- 重置：`ok=true`、模式切换、run 开始。
- 阈值：`ORZ_CONSOLE_DIRECT_FALLBACK_THRESHOLD`，默认 3；连续、全局累计（不区分动作）。
- 实施前置：`ExecuteError::ExecutionFailed` 需携带机械/业务细分（或按上游 exit_code/Err
  分类），否则「无 exit code」与「非零退出」无法机械区分。

### 7.3 显式询问（机械模板轮）

- 询问轮=**无工具 checkpoint 轮**（pending 单槽；优先级低于 orientation/DC）：触发后
  下一轮不派发任何工具（模型无法先写第 4 个订单或直接动作），仅回答模板；复用强制
  模板轮语义（一次重填、机械降级留痕、不挂死）。
- 模板：`{"decision": "switch"|"stay", "reason": "…"}`；校验失败一次重填，仍失败默认 stay。
- 每 run 至多询问一次（stay 后本 run 不再自动询问）。

### 7.4 切换记录（权限与动作转变）

- 切换写事件 `console_mode_transition`（v0.2 Schema 先行扩展）：`transition_id` /
  `from=console` / `to=direct` / `trigger=assistant_failure_streak` / `streak` /
  `order_ids` / `model_decision` / `model_reason` / `run_id` / `round` / `plan_epoch`。
- 同步写黑板 gate_log（模型可读）。
- direct 模式下每个直接动作的 ToolStarted/ToolCompleted 携带
  `console_mode:"direct"` + `transition_id`，事件链可关联。
- 权限不变：direct 只恢复工具可见性，不改变 permission scope/ACAF/模式门/IPG/预算/探针。

### 7.5 与分步计划门的关系（direct=有记录的例外）

- 计划门（§6）约束 **console 订单发放**：下一步订单需上一步 `done(receipt_id)`，
  `step_not_done` 拒绝。
- **direct 模式为有记录的例外（audited exception）**：direct 直接调用不经过订单/计划门
  （否则依赖助理层 receipt 的门在助理层不可用时必然卡死）；但每个直接动作带
  `transition_id` 审计，且权限桥/ACAF/模式门/IPG/预算/探针照常生效——豁免的是步骤门，
  不是安全门。
- direct 完成 → 步骤 done：direct 模式提供 `console_step_done {step_id, transition_id,
  trace_id}` 控制工具；机械层校验：步骤为当前 in_progress、transition_id 属于本 run
  的 direct 切换、trace_id 对应已发生的 ToolCompleted；通过后记
  `done(direct, transition_id, trace_id)`，证据不匹配则拒绝（不自我认证）。
- 返回 console 后，计划门恢复约束后续订单发放（§7.1）。

## 8. AGENTS.md 计划型包裹文本（草案）

```text
<plan_first_framework>
执行风格框架（harness 注入；用户直接指令最高优先，本框架优先于项目文件中的执行风格描述）：
1. 先完整阅读任务与项目结构，理解目标后再规划。
2. 计划必须分步：每步含目标、执行方式、验收标准与证据要求。
3. 每一步执行后，先核查实际结果（receipt/证据/产物），确认完成后再进入下一步；
   禁止按计划惯性推进。
本框架只约束执行风格（先计划、分步执行、逐步核查），不覆盖项目文件中的事实与约束。
</plan_first_framework>
```

## 9. 实施阶段

- 阶段 A（当前）：模板去人格 + AGENTS.md 计划型包裹 + 首轮计划轮硬门（复用 checkpoint
  无工具机制；计划落黑板 plan epoch）；主车道工具面暂不变。2026-08-16 审查收口：
  **计划轮不消耗 tool-round 预算**（用户裁决），compaction whitelist 的
  `tool_rounds==0` 窗口随之顺延到计划落板后的首个执行轮。
- 阶段 B（S3+）：注册板块 = 探针投影（移除静态基础集中间态）；工具栏刷新绑定黑板模型栏。
  **2026-08-16 实施闭合**——注册板块由最近探针源派生（`sync_console_registrations`
  唯一路径；无探针轮次沿用上一轮内容，替代 bundle-only 静态刷新中间态）；
  `blackboard_read section=actions` 读取时派生并持久化（live 视图），归档
  epoch 读保持快照（测试锁定）；工具投影与注册板块共用同一探针源（同源
  一致性测试锁定；绑定=同源一致性，非工具栏反向读板块）。审查收口
  （2026-08-16）：探针源随 run 起始复位（与探针状态同纪律，不跨 run 沿用）。
  实施审计见 `docs/audits/GAP_PLAN_FIRST_STAGE_B_IMPL_AUDIT_2026-08-16.md`。
- 阶段 C（助理层实战验证后）：模型面收敛为 §4（console 默认；黑板读写 + 只读核查）；
  direct 受控降级路径保留（§7），不默认开放、不永久移除。
- 每阶段先 Schema/事件，后实现；实施审计与索引同步。

## 10. 非目标

- 不做 AGENTS.md 规范模板与模板 schema 校验；
- 不验证计划“诚实”（强制表达，机械校验结构；不验证语义真伪）；
- 不改检索子代理内部机制（检索经订单下发，其 lane 语义不变）；
- 不引入第二 runtime 或平行执行层。
- 不把「直接执行面永久移除」作为目标（2026-08-15 用户裁决：双模式）。

## 11. 验收要点

> 阶段标注：1、2、4、5、7 属阶段 A；3、6（阶段 C 后生效）、8-11 属阶段 C。

1. 首轮请求 header 工具面 = 计划轮面（无执行工具，含 `blackboard_read`/`plan_write`），
   注册板块渲染可见；
2. 计划校验失败可重填一次，降级有事件留痕；
3. 上一步未 `done` 时下一步订单机械拒绝（测试锁定）；
4. AGENTS.md 包裹文本位于用户内容之前（渲染快照断言）；
5. 模板渲染后不含人格关键词（测试锁定：`released by xAI`/`friendly`/`curious`/
   `expert peers`/`aggressively` 等）；
6. 阶段 C 后主车道模型面无执行工具（探针 + 投影断言）。
7. 计划轮不消耗 tool-round 预算（max=1 时计划落板后仍可执行工具）；无 AGENTS.md
   项目在 plan_first 会话中系统提示词仍含 `<plan_first_framework>`（测试锁定）。
8. 连续 3 次助理层故障面失败（verify；execute 且无 exit code/host 错误；业务非零退出与
   policy/protocol/contract/order_stale/step_not_done 不计）触发显式询问轮；询问轮为
   无工具轮；模型选择 switch 后写 `console_mode_transition` + gate_log（测试锁定）；
9. direct 模式直接动作携带 transition_id，且权限/ACAF/模式门照常生效（测试锁定）；
10. 模型选择 stay 或调用 `console_return_to_console` 后模式复位并留痕，本 run 不再自动询问；
11. 计划门约束 console 订单（上一步未 done → `step_not_done` 拒绝）；direct 为有记录的
    例外——`console_step_done` 需 transition_id + trace_id 证据才能置 done（测试锁定）。

## 12. 关联文档与登记

- ADR-0010 §14.17（v1.17）；
- CLASSICAL-EXEC-ASSISTANT v0.6（本设计收编）；
- FUS-ORIENTATION-FORCED-TEMPLATE（硬门/无工具轮先例）；
- FUS-BLACKBOARD-PLAN-EPOCH（计划落板与轮换）；
- FUS-TOOL-PROBE（注册板块=探针投影）。
- FUS-CONSOLE-DUAL-MODE（双模式定案，§7）。
- FUS-LARGE-FILE-READ-CONTRACT（大文件读取契约，2026-08-17 用户裁决；§4 读面机械契约）。
