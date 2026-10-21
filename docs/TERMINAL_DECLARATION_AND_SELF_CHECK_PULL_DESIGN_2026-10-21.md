# 终态声明与反例自查 PULL 化设计（0dm 草案 v0.1，2026-10-21）

> **用户裁决输入（2026-10-21）**：
> 「我同意你的判定，不过常驻软提示的内容我想换成『请对非日常聊天类的任务调用反例自查』，避免严谨性遗漏。」
> 前序讨论中用户另提出：`submit` 本身是否该由模型调用、动作结束应自然、反例自查应改为模型主动走的过程并固定在工具栏。
> **2026-10-21 用户裁决二**：「我同意你的全部建议项，就按照这个进行立项吧」——§8 五点全部按推荐项定案。
>
> **状态**：**设计定稿，0dm 立项（P1，62 → 63）；S1 落码达成（2026-10-21）：loop 896/0、frontend smoke 绿；S2 UI 收口/S3 0.9.6 重建/S4 真机待续**。
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

- 判据复用既有 `run_has_substance`（`tool_rounds > 0`，或黑板有编辑记录，或 plan 存在且未完成）：
  - **有执行事实**：纯文本响应只有在 `text` 中能 `extract_model_stop` 出 `[RUN_END]` 时，才作为终答；
  - **无执行事实的短聊**：维持现状，纯文本直接终答，不要求声明（避免寒暄多付一轮）。
- 无声明且未 nudge 过：注入一次机械提示（不是 gate，不评价内容）：
  ```text
  [TERMINAL_NOTICE] 当前回复未声明结束。
  若任务已完成，请输出 [RUN_END]（intent/reason/summary）；若未完成，请继续调用工具。
  [/TERMINAL_NOTICE]
  ```
  该块注册进 injected-block filter，不持久化。
- 第二次仍无声明：**fail-open 接受**，`run_finished.terminal_source = "text_without_terminal"`，`nudged = true`，绝不挂死。
- 有声明：`terminal_source = "run_end"`，沿用既有 `intent`/`reason`/`summary` 字段。
- 预算耗尽/取消/降级等既有强制出口不变；`terminal_source` 对应 `budget_exhausted`/`cancelled`/`degraded`。
- 兼容开关：`ORZ_TERMINAL_DECLARATION`（`required` 缺省／`off` 跑分与外部客户端）。`off` 时退回旧行为，`terminal_source="text_without_terminal"` 仍记账。

### 4.2 `submit` 退役

- **模型面**：从 `controller.rs` 的工具装配移除 `submit`；`console_default` 不再注入它。主车道工具面 12 → 11。
- **机械面**：`compute_delivery_status` 保留，改为机械/UI 按需查询：
  - Web 探索器/工具栏「交付状态」按钮触发一次只读计算，渲染到 plan 视图或侧栏；
  - 不再要求模型调用、不再有两阶段确认、不再置 plan 末步 done。
- **plan 末步**：`planning::TERMINAL_STEP_IDS` 新计划收窄为 `["deliver"]`；旧计划/旧 journal 中的 `submit` 末步仍按 terminal 解释（回放兼容），但新计划不再生成。
- **旧事件**：`tool_completed{tool:"submit"}` 保留解释；`submit_not_current`/`submit_lane_denied` 等拒绝面仅作历史回放，不再生产。
- **兼容**：`ORZ_SUBMIT_TOOL=1` 可临时恢复旧工具（仅 console 车道），缺省关；跑分/外部客户端不受影响。
- **README**：删除「任务最终经 submit 两阶段交付」与运行路径里的「submit 两阶段交付」；改为「终态用 `[RUN_END]`；交付状态由 UI 按需展示」。

### 4.3 `self_check` PULL

- **模型工具**：新增无参 `self_check`（主车道，含 console；检索/grill 不装配）。调用返回三问模板并落 `tool_completed{tool:"self_check"}`，不阻断、不要求继续：
  ```text
  [SELF_CHECK]
  请对即将输出的结论做最后一次反例自查：
  1. 是否存在未验证的前提？
  2. 是否存在可推翻结论的已知证据？
  3. 结论强度是否超出证据支持？
  这不是硬门。请据此修订或确认终答，然后按 §4.1 输出 [RUN_END]。
  [/SELF_CHECK]
  ```
  三问文本与现有 `COUNTEREXAMPLE_GATE_BLOCK` 单一来源，改名 `SELF_CHECK_BLOCK`；旧 gate 事件类型保留回放，不再生产。
- **工具栏**：Web 工具栏固定「反例自查」按钮（TUI 加同义快捷键）：
  - 运行中：经 `_orz/queue_append` 投递补充消息「请对当前结论执行反例自查（三问），然后按需继续或输出 [RUN_END]。」——走既有中途补充通道，不新增 ACP 方法；
  - 空闲：按钮禁用并提示「先发起任务」，避免凭空起一轮。
- **自动注入退役**：删除终答候选处的 `[COUNTEREXAMPLE_GATE]` 注入与 `counterexample_fired` 分支；`answer_gate_has_substance` 改名为 `run_has_substance`，只服务 §4.1 的终态判据。
- **审计**：`run_finished` 新增 `self_check_called`（bool），由本 run 的 `tool_completed{tool:"self_check"}` 派生；不新增事件族，不要求模型举证。
- **常驻软提示**（用户指定文案，逐字）：
  ```text
  请对非日常聊天类的任务调用反例自查。
  ```
  放在主车道 resident 行（与 `model_stop_resident_line` 同区，`model_face.rs`），只出现一次；不进入对话侧车、不进归档。检索/grill 车道不出现。

### 4.4 契约面

| 面 | 变更 | 同步要求 |
|---|---|---|
| 工具面 | `submit` 移除、`self_check` 新增（净 12 不变） | `orz-host/src/tools.rs` 保留面计数、README、controller 测试 |
| `run_finished` | 新增 `terminal_source`（闭集：`run_end`/`text_without_terminal`/`short_text`/`budget_exhausted`/`cancelled`/`degraded`）＋`self_check_called`（bool）＋`nudged`（bool） | JSON schema、Python 冻结镜像、Rust 法官、fixtures 四同步；旧 payload 不加字段时按旧形解释 |
| 事件面 | `counterexample_gate` 不再生产（保留解释）；`self_check` 不新增事件族，走 `tool_*` | journal-conformance 允许旧事件回放 |
| plan | 新计划 terminal step 仅 `deliver`；旧 `submit` 末步兼容 | planning 单测、恢复路径测试 |
| README | 删 submit 交付表述；增终态/自查 PULL | 中英双语 |

### 4.5 交互时序（主车道有执行事实）

```text
模型：...进度文本（无工具调用，无 [RUN_END]）
loop ：注入 [TERMINAL_NOTICE]（一次）
模型：继续工具 或 输出 [RUN_END]
        ├─ 继续工具 → 正常循环
        └─ [RUN_END] → 落 run_finished{terminal_source:"run_end", ...}
模型再给无声明文本（第二次）→ fail-open 接受，terminal_source="text_without_terminal", nudged=true

模型想自查：调用 self_check（或用户点工具栏）
        → 三问模板返回 → 模型修订/确认 → [RUN_END]
```

## §5 为什么不是「submit 硬门」

| 方案 | 结果 |
|---|---|
| **submit 硬门** | 每个主车道任务多两轮工具调用；短聊也付；检索/grill 无 submit 无法收尾；与 ADR O1「非硬门」冲突 |
| **`[RUN_END]` 终态（本稿）** | 模型已有该语法；不新增工具即可表达结束；短聊豁免；无声明时一次 nudge 后 fail-open，不挂死 |
| **保留现状** | 「进度文本＝终答」无法区分，严谨性缺口仍在；README 继续误导 |

`submit` 退役不等于「交付状态」消失：机械层仍能算，UI 仍能展示，只是不再让模型为机械信息付两轮。

## §6 测试矩阵（S2）

1. 主车道有执行事实、纯文本无 `[RUN_END]` → 注入一次 `[TERMINAL_NOTICE]`，不落终态；第二次接受，`terminal_source=text_without_terminal`。
2. 主车道有执行事实、文本带 `[RUN_END]` → 直接终答，`terminal_source=run_end`，不 nudge。
3. 短聊（0 工具轮、无编辑、无未完成 plan）→ 直接终答，`terminal_source=short_text`，不要求声明。
4. `self_check` 调用 → `tool_completed` 落账，`run_finished.self_check_called=true`；未调用为 false；无 gate 事件。
5. `submit` 不在工具列表；旧 journal 含 submit 事件可回放；`ORZ_SUBMIT_TOOL=1` 时 console 车道恢复旧工具。
6. 新 plan 的 `submit` 末步被拒；旧 plan 的 `submit` 末步按 terminal 解释。
7. `ORZ_TERMINAL_DECLARATION=off` → 旧行为，`terminal_source=text_without_terminal`。
8. 检索/grill 车道：无 `self_check`、无 terminal notice、行为不变。
9. Web 工具栏「反例自查」运行中投递补充；空闲禁用；TUI 快捷键同义。
10. README/工具计数/运行路径断言与代码一致。

## §7 批序与计数

- **S0**：本设计稿，用户裁决 §8 五点。
- **S1**：核心落码（终态判据、nudge、`self_check`、submit 退役、run_finished 字段）＋单测。
- **S2**：UI（Web 工具栏、TUI 键、交付状态按需查询）＋前端烟测。
- **S3**：0.9.6 双平台重建＋装机位换装＋身份门。
- **S4**：三会话真机（短聊/长任务进度文本/显式自查）＋旧 journal 回放。
- **计数**：建议立项 `0dm`（P1，62 → 63）；S1 落码后仍 `partial`，S4 收口才翻 `implemented`。

- **S1 落码达成（2026-10-21，orz `19638ee2`，17 文件 `+410/−177`，未推送）**：
  - 终态：`agent_loop.rs` 主车道有执行事实（`run_has_substance`）时，纯文本须 `extract_model_stop` 出 `[RUN_END]`；无声明先注入一次 `TERMINAL_NOTICE_BLOCK`，第二次 fail-open；短聊 `terminal_source=short_text`。
  - 自查：`prompt.rs` 新增 `SELF_CHECK_BLOCK`/`SELF_CHECK_RESIDENT_LINE`/`TERMINAL_NOTICE_BLOCK`；`self_check` 工具（ReadOnly 类）在 controller 主面装配；`tool_run.rs` 返回三问模板并落 `tool_completed`；Web 工具栏「反例自查」经 `queueSupplement` 投递。
  - submit：controller 默认不再装配；`with_legacy_submit_tool`/`ORZ_SUBMIT_TOOL=1` 兼容旧 console；`planning` 新计划 terminal 收窄为 `deliver`，旧 `submit` 末步仍由 `LEGACY_TERMINAL_STEP_IDS` 解释。
  - 落账：`run_finished` 增 `terminal_source`/`terminal_nudged`/`self_check_called`；旧 gate 仅 `ORZ_TERMINAL_DECLARATION=off` 且 controller 未启用终态声明时保留（测试默认路径）。
  - 验证：`orz-loop` **896/0**（3 ignored）＋`orz-web` **52/0**＋`orz-bin --bins` check 通过＋frontend smoke 全过；`orz-host` 385/4（4 项进程/符号链接环境用例，与 0dm 无关）。
  - S2 剩余：TUI 快捷键、交付状态 UI 面板；S3 0.9.6 重建；S4 真机三场景。

## §8 裁决（2026-10-21 用户全部同意，按推荐项定案）

1. **工具面**：`submit` 移除＋`self_check` 新增（净 12）——**定案**。
2. **终态范围**：仅主车道「有执行事实」要求 `[RUN_END]`，短聊豁免——**定案**。
3. **默认开关**：`ORZ_TERMINAL_DECLARATION=required` 缺省、跑分/外部客户端显式 `off`——**定案**；代码以生产 builder 显式置 required，测试默认关以保留旧文本终答钉子。
4. **工具栏**：运行中投递补充、空闲禁用——**定案**。
5. **立项计数**：0dm，62 → 63——**定案**。

> 兼容注记（S1 实施口径）：`ORZ_TERMINAL_DECLARATION=off` 或 controller 未启用终态声明时，旧反例门保留为兼容路径（测试默认路径）；生产 required 路径不再触发旧门。

## §9 不做与风险

- **不做**：机械层判定「是否完成」、不设证据门槛、不驳回 `[RUN_END]`；不新增自动注入；不改墙钟/预算/资源/取消强制权；不要求所有车道都声明终态。
- **风险①：模型从不调用 `self_check`** → 常驻软提示＋工具栏；`run_finished.self_check_called=false` 可统计，不加硬门。
- **风险②：模型提前写 `[RUN_END]`** → 这是模型的判断（P1）；`intent/reason/summary` 分布可观测，机械层不驳回。
- **风险③：外部客户端不写 `[RUN_END]`** → 兼容开关 `off`；默认 required 仅主车道有执行事实。
- **风险④：移除 submit 影响 console/plan 旧流程** → `ORZ_SUBMIT_TOOL=1` 兼容；旧 journal 回放不变；新计划 terminal step 收窄为 `deliver`。
- **风险⑤：README/测试漂移** → S1 同批更新中英文 README、工具计数、运行路径断言、schema/verifier/Python 镜像。

## §10 关键词

0dm、终态声明、`[RUN_END]`、submit 退役、自检 PULL、`self_check`、工具栏反例自查、常驻软提示、`terminal_source`、`text_without_terminal`、短聊豁免、`ORZ_TERMINAL_DECLARATION`、0.9.6、62 → 63。
