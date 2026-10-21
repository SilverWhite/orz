# 终态声明与反例自查 PULL 化设计（0dm 草案 v0.1，2026-10-21）

> **用户裁决输入（2026-10-21）**：
> 「我同意你的判定，不过常驻软提示的内容我想换成『请对非日常聊天类的任务调用反例自查』，避免严谨性遗漏。」
> 前序讨论中用户另提出：`submit` 本身是否该由模型调用、动作结束应自然、反例自查应改为模型主动走的过程并固定在工具栏。
> **2026-10-21 用户裁决二**：「我同意你的全部建议项，就按照这个进行立项吧」——§8 五点全部按推荐项定案。
>
> **状态**：**设计定稿 v0.2（2026-10-21 审查修订）**；0dm 立项（P1，62 → 63）；S1 落码＋审查修复达成（loop 908/0、assurance 304/0、TUI 186/0、web 52/0、frontend smoke 全过；S2 UI 收口/S3 0.9.6 重建/S4 真机待续）。v0.1 的「一次 nudge」改为按工具轮 episode 重挂、最多 2 次；`run_finished`/`gate_decision` schema 已四同步；详见 §11 修订记录。
> **关系**：本稿修订 [ADR-0010 §14.35 O1](../adr/ADR-0010-vol-14-addenda-index.md)（submit 非硬门）与 §14.76（反例门），
> 承接 [`NEUTRAL_TERMINATION_AND_MODEL_STOP_DESIGN_2026-09-22.md`](NEUTRAL_TERMINATION_AND_MODEL_STOP_DESIGN_2026-09-22.md)
> 的 P1/P2（判断归模型、机械层只记录与传话），并把 0.9.5 已修的 gate 重复问题推进到机制层。

## §1 一句话设计

**终态由模型显式声明 `[RUN_END]`，不再由「没有工具调用」暗判；`submit` 从模型工具面退役，交付状态改为机械/UI 按需查询；反例自查从终答前自动注入改为模型/用户主动拉取的 `self_check`；常驻软提示固定为「请对非日常聊天类的任务调用反例自查。」**

## §2 问题（现场读数）

1. **「无工具调用＝终答」是传输层信号，不是任务层信号。** 模型写一段进度说明、没有调工具，loop 就把它当终答；这正是「提前停止动作」的可感来源。0.9.5 的 gate 自查跳过只消除了重复终答，没有改变这条判据。
2. **`submit` 名不副实。** 它不携带终答，只是两阶段工作区变更清单/交付状态渲染；ADR 0g 已明确「信息展示、非硬门」，但 [README.md](../README.md) 第 62/89 行仍写成「任务最终经 submit 两阶段交付」，构成文档漂移，也直接塑造了用户「终答应该由 submit 触发」的预期。
3. **反例门是 PUSH 机制。** 终答候选处自动注入 `[COUNTEREXAMPLE_GATE]`，模型被动应答；模型从 run 6 起学会在候选里自带「反例自查」，门又重复注入（322 §5 时间线）。把自查改成 PULL，既保留严谨性，又消除「机械层替模型决定何时自查」。
4. **`[RUN_END]` 已是现成契约。** 模型提示里已有常驻语法行，模型在长任务终答里本来就会写 `intent`/`reason`/`summary`；现在机械层只把它记进 `run_finished`，不参与终止。把它升级为终态信号，不需要新造模型面协议。
5. **工具面现状。** 主车道生产装配 12 工具（含 `submit`）；`submit` 只在 `console_default_enabled` 时加入，检索/grill 车道没有。移除它不会影响检索车道；新增 `self_check` 可保持 12 工具面不变。

## §3 设计原则

- **P1 判断归模型**：结束与否、何时自查，由模型判断；机械层不判定「是否完成」、不驳回 `[RUN_END]`。
- **P2 机械层只记录与传话**：机械层只做终态信号识别、nudge 一次、落账；不评价内容、不要求举证。
- **P3 回应归外部**：nudge 不是「驳回」，最终 fail-open；外部（人/编排方）据 `run_finished` 的 `terminal_source` 决定是否续跑。
- **P4 无应答者不剥夺出口**：无头/跑分车道仍可用 `[RUN_END]` 结束；兼容开关可关掉声明要求。
- **P5 PULL 优先**：自查由模型/用户主动拉取；常驻只留一行软提示，不做自动注入。

## §4 设计

### 4.1 终态声明（主车道）

- 判据＝**本 run** 执行事实（v0.2 修订）：`tool_rounds > 0`，或黑板编辑记录带本 run 章（`EditRecord.run == current_run_id`），或本 run 起点后 plan 新增/变更且仍有未完成步骤。上一 run 的 edits、恢复的未完成 plan、`with_plan` 预置的旧 plan 不再单独触发声明；会话级状态不再击穿短聊豁免。
  - **有执行事实**：纯文本响应只有在 `text` 中能 `extract_model_stop` 出 `[RUN_END]`（行级、独占行、fence 感知）时，才作为终答；
  - **无执行事实的短聊**：纯文本直接终答，不要求声明（`terminal_source=short_text`）。
- 无声明且本候选 episode 尚未提示：注入一次机械提示（不是 gate，不评价内容）：
  ```text
  [TERMINAL_NOTICE]
  上文是你的草稿。若任务已完成，请只输出 [RUN_END] 声明（intent/reason/summary），不要重复正文；若未完成，请继续调用工具。
  [/TERMINAL_NOTICE]
  ```
  该块注册进 injected-block filter（前缀＋生成形态），**不持久化**；候选文本先作为 assistant 轮提交，模型下一轮能看到草稿，只需补声明或继续工具。
- **nudge 预算 v0.2**：按候选 episode 计，最多 `TERMINAL_NUDGE_MAX=2` 次；每次工具轮产生新动作后才重新获得一次提示；用尽后 fail-open 接受，`run_finished.terminal_source="text_without_terminal"`、`terminal_nudged=true`、`terminal_nudge_count=2`，绝不挂死。没有新工具动作的第二次无声明候选直接接受（不再无限加轮）。
- 有声明：`terminal_source="run_end"`，`terminal_declaration` 存入 `LoopOutcome`；终答正文**剥离**声明块后交付，`intent`/`reason`/`summary` 从声明落账。
- 预算耗尽/取消/降级等既有强制出口不变；`run_finished.terminal_source` 只覆盖 `run_finished` 出口：`run_end`/`text_without_terminal`/`short_text`/`budget_exhausted`/`gate_block`/`close_round`/`unspecified`。取消走 `run_cancelled`、日志降级走 `run_terminated`，二者**不携带** `terminal_source`（v0.1 闭集里的 `cancelled`/`degraded` 删除）。
- 兼容开关：`ORZ_TERMINAL_DECLARATION`（`required` 缺省／`off` 跑分与外部客户端）。`off` 时退回旧反例门；有本 run 执行事实时 `terminal_source="text_without_terminal"`，无执行事实才是 `short_text`。旧门条件与声明要求共用解析后的 `controller && env`，避免「既不要求也不注门」。

### 4.2 `submit` 退役

- **模型面**：从 `controller.rs` 的工具装配移除 `submit`；`console_default` 不再注入它。净工具面仍 12（移除 submit＋新增 `self_check`）；非 console 主车道为 11（无 submit/self_check 之外的旧面差异）。
- **机械面**：`compute_delivery_status` 保留，改为机械/UI 按需查询：
  - Web 探索器/工具栏「交付状态」按钮触发一次只读计算，渲染到 plan 视图或侧栏；
  - 不再要求模型调用、不再有两阶段确认；`submit` handler 也按 `submit_tool_enabled()`（builder 或 `ORZ_SUBMIT_TOOL=1`）拒绝，退役后幻觉调用不产生交付事件。
- **plan 末步**：`planning::TERMINAL_STEP_IDS` 新计划收窄为 `["deliver"]`；旧计划/旧 journal 中的 `submit` 末步仍按 terminal 解释（回放兼容），但新计划不再生成。`deliver` 由 `[RUN_END]`（intent=conclude、reason=completed）机械置 Done；其他 reason 不推进。
- **旧事件**：`tool_completed{tool:"submit"}` 保留解释；`submit_not_current`/`submit_lane_denied` 等拒绝面仅作历史回放，不再生产（新增 `submit_retired` 拒绝码用于退役后的幻觉调用）。
- **兼容**：`ORZ_SUBMIT_TOOL=1` 可临时恢复旧工具（仅 console 车道），缺省关；跑分/外部客户端不受影响。
- **README**：删除「任务最终经 submit 两阶段交付」与运行路径里的「submit 两阶段交付」；改为「终态用 `[RUN_END]`；交付状态由 UI 按需展示」。

### 4.3 `self_check` PULL

- **模型工具**：新增无参 `self_check`（主车道，含 console 与 `--plan` 生产入口；检索/grill 不装配）。调用返回三问模板并落 `tool_completed{tool:"self_check"}`，不阻断、不要求继续；handler 只允许 `self_check_enabled()` 车道且 `activation_id=None`，检索子代理调用按 `self_check_lane_denied` 拒绝。
  ```text
  [SELF_CHECK]
  请对即将输出的结论做最后一次反例自查：
  1. 是否存在未验证的前提？
  2. 是否存在可推翻结论的已知证据？
  3. 结论强度是否超出证据支持？
  这不是硬门。请据此修订或确认终答，然后按 §4.1 输出 [RUN_END]。
  [/SELF_CHECK]
  ```
  三问正文由 `SELF_CHECK_QUESTIONS` 常量测试钉住与旧 gate 同文；旧 gate 事件类型保留回放，生产 required 路径不再生产。
- **工具栏**：Web 工具栏固定「反例自查」按钮（TUI `Ctrl+G` 同义快捷键）：
  - 运行中：经 `_orz/queue_append` 投递补充消息 `SELF_CHECK_SUPPLEMENT`「请对当前结论执行反例自查（三问），然后按需继续或输出 [RUN_END]。」——走既有中途补充通道，不新增 ACP 方法；
  - 空闲：按钮禁用（`syncToolbar` 按 `state.running` 同步），点击只提示「先发起任务」，避免凭空起一轮。
- **自动注入退役**：required 路径删除终答候选处的 `[COUNTEREXAMPLE_GATE]` 注入与 `counterexample_fired` 分支；`answer_gate_has_substance` 改名 `run_has_substance`，只服务 §4.1 的终态判据。`off`/controller-not-enabled 兼容路径仍保留旧门。
- **审计**：`run_finished` 新增 `self_check_called`（bool），由本 run 的 `tool_completed{tool:"self_check"}` 派生；不新增事件族，不要求模型举证。
- **常驻软提示**（用户指定文案，逐字）：
  ```text
  请对非日常聊天类的任务调用反例自查。
  ```
  放在主车道**系统提示**（`agent_loop.rs`，与 `model_stop_resident_line` 同区语义），由 `LoopProfile::self_check_prompt` 控车道；只出现一次；不进入对话侧车、不进归档。检索/grill 车道不出现（grill 的 role 也是 Main，不得再用 role 判断）。

### 4.4 契约面

| 面 | 变更 | 同步要求 |
|---|---|---|
| 工具面 | `submit` 移除、`self_check` 新增（净 12 不变） | `orz-host/src/tools.rs` 保留面计数、README、controller 测试 |
| `run_finished` | 新增 `terminal_source`（闭集：`run_end`/`text_without_terminal`/`short_text`/`budget_exhausted`/`gate_block`/`close_round`/`unspecified`）＋`terminal_nudged`（bool）＋`terminal_nudge_count`（int）＋`self_check_called`（bool，由成功 `tool_completed` 派生） | JSON schema、Python 冻结镜像、Rust 法官、fixtures 四同步（S1 修订批已做）；旧 payload 仍按旧形解释 |
| 事件面 | `counterexample_gate` 不再生产（保留解释）；`self_check` 不新增事件族，走 `tool_*` | journal-conformance 允许旧事件回放 |
| plan | 新计划 terminal step 仅 `deliver`；旧 `submit` 末步兼容 | planning 单测、恢复路径测试 |
| README | 删 submit 交付表述；增终态/自查 PULL | 中英双语 |

### 4.5 交互时序（主车道有执行事实）

```text
模型：...进度文本（无工具调用，无 [RUN_END]）
loop ：把草稿入 assistant 轮 + 注入 [TERMINAL_NOTICE]（第 1 次）
模型：继续工具 或 只输出 [RUN_END] 声明
        ├─ 继续工具 → 正常循环；下一次无声明候选可再提示（最多 2 次）
        ├─ 只输出声明 → 复用草稿作为终答，落 run_finished{terminal_source:"run_end"}
        └─ 无声明且无新工具 → fail-open 接受，terminal_source="text_without_terminal"
用尽 2 次 nudge 后再无声明 → fail-open（text_without_terminal），绝不挂死

模型想自查：调用 self_check（或用户点工具栏 / TUI Ctrl+G）
        → 三问模板返回 → 模型修订/确认 → [RUN_END]
```

## §5 为什么不是「submit 硬门」

| 方案 | 结果 |
|---|---|
| **submit 硬门** | 每个主车道任务多两轮工具调用；短聊也付；检索/grill 无 submit 无法收尾；与 ADR O1「非硬门」冲突 |
| **`[RUN_END]` 终态（本稿）** | 模型已有该语法；不新增工具即可表达结束；短聊豁免；无声明时按工具轮 episode 最多 2 次 nudge 后 fail-open，不挂死 |
| **保留现状** | 「进度文本＝终答」无法区分，严谨性缺口仍在；README 继续误导 |

`submit` 退役不等于「交付状态」消失：机械层仍能算，UI 仍能展示，只是不再让模型为机械信息付两轮。

## §6 测试矩阵（S1 修订批已覆盖 1/2/3/4/5/6/7/8/9；10 为文档/计数断言）

1. 主车道有本 run 执行事实、纯文本无 `[RUN_END]` → 注入 `[TERMINAL_NOTICE]`（草稿入 assistant），工具轮后可再提示一次；用尽 2 次后 fail-open，`terminal_source=text_without_terminal`、`terminal_nudge_count=2`。
2. 主车道有执行事实、文本带 `[RUN_END]` → 终答正文剥离声明，`terminal_source=run_end`，不 nudge；声明-only 的 nudge 回复复用草稿，不重写全文。
3. 短聊（0 工具轮、无本 run 编辑、无本 run plan 变更）→ 直接终答，`terminal_source=short_text`，不要求声明；恢复会话中上一 run 的 edits/plan 不再触发。
4. `self_check` 成功完成 → `tool_completed` 落账且 `self_check_called=true`；被拒绝/被 IPG 拦截为 false；检索 lane 不得装配/执行。
5. `submit` 不在工具列表；旧 journal 含 submit 事件可回放；`ORZ_SUBMIT_TOOL=1` 时 console 车道恢复旧工具；默认 handler 按 `submit_retired` 拒绝。
6. 新 plan 的 `submit` 末步被拒；旧 plan 的 `submit` 末步按 terminal 解释；新 plan 的 `deliver` 由 run_end(completed) 置 Done。
7. `ORZ_TERMINAL_DECLARATION=off` → 旧反例门；有本 run 执行事实时 `terminal_source=text_without_terminal`，纯短聊才 `short_text`。
8. 检索/grill 车道：无 `self_check`、无 terminal notice、无声明要求；grill 系统提示不出现常驻行。
9. Web 工具栏「反例自查」运行中投递补充、空闲 disabled；TUI `Ctrl+G` 同义。
10. README/工具计数/运行路径断言与代码一致；`run_finished`/`gate_decision` schema 四同步由 conformance 测试钉住。

## §7 批序与计数

- **S0**：本设计稿，用户裁决 §8 五点。
- **S1**：核心落码（终态判据、nudge、`self_check`、submit 退役、run_finished 字段）＋单测。
- **S2**：UI（Web 工具栏、TUI 键、交付状态按需查询）＋前端烟测。
- **S3**：0.9.6 双平台重建＋装机位换装＋身份门。
- **S4**：三会话真机（短聊/长任务进度文本/显式自查）＋旧 journal 回放。
- **计数**：建议立项 `0dm`（P1，62 → 63）；S1 落码后仍 `partial`，S4 收口才翻 `implemented`。

- **S1 落码达成（2026-10-21，orz `19638ee2`，17 文件 `+410/−177`）**：
  - 终态：`agent_loop.rs` 主车道有本 run 执行事实时，纯文本须 `extract_model_stop` 出 `[RUN_END]`；无声明按工具轮 episode 最多 2 次 `TERMINAL_NOTICE`，之后 fail-open；短聊 `terminal_source=short_text`。
  - 自查：`prompt.rs` 新增 `SELF_CHECK_BLOCK`/`SELF_CHECK_RESIDENT_LINE`/`TERMINAL_NOTICE_BLOCK`；`self_check` 工具（ReadOnly 类）在 controller 主面装配；`tool_run.rs` 返回三问模板并落 `tool_completed`；Web 工具栏经 `queueSupplement` 投递，TUI `Ctrl+G`。
  - submit：controller 默认不再装配；`with_legacy_submit_tool`/`ORZ_SUBMIT_TOOL=1` 兼容旧 console；`planning` 新计划 terminal 收窄为 `deliver`，旧 `submit` 末步仍由 `LEGACY_TERMINAL_STEP_IDS` 解释；`deliver` 由 run_end(completed) 完成。
  - 落账：`run_finished` 增 `terminal_source`/`terminal_nudged`/`terminal_nudge_count`/`self_check_called`；旧 gate 仅解析后 `controller && env` 为 false 时保留。
  - 验证：`orz-loop` **908/0**（3 ignored）＋`orz-assurance` **304/0**＋`orz-tui` **186/0**＋`orz-web` **52/0**＋frontend smoke 全过；`orz-host` 384/5（进程/Job/符号链接/截断环境用例；`run_terminal_cmd_truncation` 单独重跑通过，与 0dm 无关）。
  - 已修审查项：schema 四同步、nudge 重挂/上限、run 作用域、notice 不落侧车、env 矩阵、`--plan` 开关、grill/检索隔离、草稿复用、`RUN_END` 行级 fence 解析与正文剥离、`self_check_called` 完成派生、`submit` handler 退役、`deliver` 完成源、Web 空闲禁用、TUI 快捷键。
  - S2 剩余：交付状态 UI 面板；S3 0.9.6 重建；S4 真机三场景。

## §8 裁决（2026-10-21 用户全部同意，按推荐项定案）

1. **工具面**：`submit` 移除＋`self_check` 新增（净 12）——**定案**。
2. **终态范围**：仅主车道「有执行事实」要求 `[RUN_END]`，短聊豁免——**定案**。
3. **默认开关**：`ORZ_TERMINAL_DECLARATION=required` 缺省、跑分/外部客户端显式 `off`——**定案**；代码以生产 builder 显式置 required，测试默认关以保留旧文本终答钉子。
4. **工具栏**：运行中投递补充、空闲禁用——**定案**。
5. **立项计数**：0dm，62 → 63——**定案**。

> 兼容注记（S1 v0.2 实施口径）：声明要求＝`controller.terminal_declaration_required() && ORZ_TERMINAL_DECLARATION`。任一为 false 时保留旧反例门；有本 run 执行事实时 `terminal_source=text_without_terminal`，纯短聊才 `short_text`。四格矩阵：true/true=新路径；true/false、false/true、false/false=旧门兼容。生产 builder 显式 true；`--plan` 生产入口也已补齐。

## §9 不做与风险

- **不做**：机械层判定「是否完成」、不设证据门槛、不驳回 `[RUN_END]`；不新增自动注入；不改墙钟/预算/资源/取消强制权；不要求所有车道都声明终态。
- **风险①：模型从不调用 `self_check`** → 常驻软提示＋工具栏；`run_finished.self_check_called=false` 可统计，不加硬门。
- **风险②：模型提前写 `[RUN_END]`** → 这是模型的判断（P1）；`intent/reason/summary` 分布可观测，机械层不驳回。
- **风险③：外部客户端不写 `[RUN_END]`** → 兼容开关 `off` 会保留旧门；默认 required 仅主车道有本 run 执行事实时要求。
- **风险④：移除 submit 影响 console/plan 旧流程** → `ORZ_SUBMIT_TOOL=1` 兼容；旧 journal 回放不变；新计划 terminal step 收窄为 `deliver`，由 run_end(completed) 完成。
- **风险⑤：README/测试漂移** → S1 v0.2 已同步中英文 README、schema/fixture/Python/Rust judge、工具计数与运行路径；`check_repository.py` 与 manifest 随后重生成。

## §11 审查修订记录（v0.2，2026-10-21）

独立设计/实现/符合性审查后，S1 补丁处理如下：

- **契约**：`run_finished` schema 增 `terminal_source`（7 值闭集）/`terminal_nudged`/`terminal_nudge_count`/`self_check_called`；`gate_decision.decision` 增 `nudge`；fixture、Rust conformance 测试、Python 测试四同步。
- **状态机**：nudge 改为按工具轮 episode 重挂、最多 2 次；草稿入 assistant 轮，声明-only 回复复用草稿；终答剥离声明块；解析改为行级 + fence 感知，未闭合后文不再吞掉前文合法声明。
- **作用域**：`run_has_substance` 用本 run `EditRecord.run` + run 起点 plan 快照；恢复会话的旧 edits/plan 不再触发。
- **落账**：`self_check_called` 由成功 `tool_completed` 派生；`terminal_source` 删除不可达的 `cancelled`/`degraded`，补 `gate_block`/`close_round`/`unspecified`。
- **隔离**：`LoopProfile` 增 `terminal_declaration`/`self_check_prompt` 车道位；grill/检索 false；子代理投影剔除 `self_check`；handler 增 activation 拒绝。
- **兼容**：`declaration_required = controller && env`；`--plan` 生产入口补 required；`submit` handler 按 `submit_tool_enabled()` 拒绝。
- **完成源**：`deliver` 由 run_end(conclude/completed) 机械置 Done；`submit` 退役不再推进。
- **UI**：Web 空闲禁用；TUI `Ctrl+G`；`plan_write` 描述收窄为 deliver。
- **验证**：loop 908/0、assurance 304/0、TUI 186/0、web 52/0、frontend smoke 全过、schema 测试通过。

## §10 关键词

0dm、终态声明、`[RUN_END]`、submit 退役、自检 PULL、`self_check`、工具栏反例自查、常驻软提示、`terminal_source`、`text_without_terminal`、`terminal_nudge_count`、短聊豁免、`ORZ_TERMINAL_DECLARATION`、0.9.6、62 → 63。
