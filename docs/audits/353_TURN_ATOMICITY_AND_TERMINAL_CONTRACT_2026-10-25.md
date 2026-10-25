# 353 批：回合原子性断链与终态声明契约审查

> 日期：2026-10-25
> 用户令：那个活跃会话上下文实际缺失；run_end 这个机制挺弱智的，这个设计不好，我考虑得改；请给本轮 bug 审查和挖掘落一份审计文档。
> 范围：① LIF 工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹` 会话 `51da03ea`（10-23 至 10-25）的只读取证——run 中断、侧车不落盘、上下文回退三段链路；② 按用户 2026-10-25 裁决落码：时间基准统一北京时间、终态声明缺省关闭、RLI 推送面删除（§9）。**已改动源码、未提交**。
> 计数：65 不变（审查件；未立项）。

## §0 结论

**用户的"上下文实际缺失"不是压缩造成的，是 run 中断造成的。** `RUN-51da03ea-11` 已完成 P-LIT6-04 harness 修复并把 §11 写入 `BRANCH_DISCUSSION_LITERATURE_R9_2026-10-22.md`（`tool_completed` exit 0），随后发起写 §12 的命令、用户批准、`tool_started` 后**事件流终止**——无 `tool_completed`、无 `run_finished`、无 `run_cancelled`、无 `run_invalidated`。侧车只在 run 终态落盘，于是这一轮 11 个 model 轮全部蒸发；下一次 run 从 run 9 的现场（R9 §10、13022 字符）起步，表现为"上下文不连贯、停在 10 节"。

**磁盘前进了，对话没前进**：R9 文件现在 16,501 字符、`## 11` 四小节在盘（mtime `2026-10-25 12:52:39`，与 `tool_completed 04:52:39+00:00` 逐秒吻合）；而侧车里 `16499`／`断线前那次补跑`／`队列收尾` **命中 0 次**，最后一条 assistant 仍是"已追加到 R9 §10"。

**同型三次**：`RUN-10/11/12` 全部是 `permission_decision` → `tool_started` → 事件流终止，全部卡在 `run_terminal_cmd` 执行长 `@'…'@` heredoc 时。不是墙钟、不是轮预算、不是 RUN_END 解析失败——是**工具挂死 + run 级持久化**的叠加。

**终态声明机制被判为设计缺陷（P1）**：它把"回合边界"和"任务终态"混在一个需要模型逐字记住格式的文本块里，失败模式是静默的、代价是丢一轮交付。本批给出"零声明默认 / 声明降为不阻断元数据 / 机器可读终态工具化 / 删机械催收"四件套建议（§5）。

## §1 取证：RUN-51da03ea-11 时间线

时间戳来源＝`orz_assurance::journal::recorder` 的 `chrono::Utc::now().to_rfc3339()`（`crates/orz-assurance/src/journal/recorder.rs:230-232`）——**本机系统时钟**，以 UTC（`+00:00`）序列化，**不是网络时间**。下表左列为文件原值，右列为本地时间（UTC+8）。

| UTC | 本地 | 事件 |
|---|---|---|
| 04:49:58.838 | 12:49:58 | `run_started`（run 目录 mtime 同为 12:49:58） |
| 04:52:29 | 12:52:29 | `model_output` 工具调用，命令体含 `## 11` |
| 04:52:38 → 04:52:40 | 12:52:38 → 12:52:40 | `tool_started` → **`tool_completed` exit 0**（R9 落盘） |
| 04:52:50 | 12:52:50 | `model_output` **文本**：「断线前那次补跑其实已经跑完并落盘了……R9 现在 16499 字符 / 368 行，新增的是 §11」——**未带 `[RUN_END]`** |
| 04:52:51 | 12:52:51 | `face_fingerprint` round 9，131 条（该文本作为一条 assistant 消息留在窗口，回合继续） |
| 04:53:05 → 04:53:39 | 12:53:05 → 12:53:39 | P-LIT9-02/03 脚本，`tool_completed` exit 0（wall 30,037 ms） |
| 04:53:53 → 04:54:17 | 12:53:53 → 12:54:17 | P-LIT9-03 ±10% 矩阵，`tool_completed` exit 0 |
| 04:54:31 | 12:54:31 | `model_output` 工具调用，命令体含 `## 12. 队列收尾` → `permission_requested` |
| **04:55:00.815** | **12:55:00** | `permission_decision` **allow_once**（用户点击） |
| **04:55:00.828** | **12:55:00** | `tool_started` `call_00_WokxxG8vgrbOzQhNePSO9268` |
| — | — | **事件流终止**（`events.jsonl` 末行；run 目录内无其他文件） |

- run 11 规模：122 个事件、11 个 model 轮、14 次工具调用；最后一次 `face_fingerprint` 为 135 条 / 129,913 字符。
- 该轮**已交付的成果**：R9 §11（`## 11` + 11.1–11.4 四小节）；`## 12` 那次写入**从未执行**（文件内无 §12）。
- 该轮**未落盘的成果**：04:52:50 的答复文本、04:53:39/04:54:17 两次脚本的读数与结论、§12 的草稿。

## §2 三层根因

**L1 执行层（工具挂死）**：`tool_started` 之后没有任何 `tool_completed`。三次事故的命令体都是 PowerShell here-string（`@'…'@ | Set-Content`）写长文档；here-string 未闭合或编码吃掉引号时，PowerShell 转入续行等待 stdin，工具永不返回。事件流里表现为"启动即静默"。

**L2 持久化层（run 级落盘）**：`persist_conversation_sidecar` 的调用点只有 run 终态族——`acp_server.rs:3642`（close/headless 收尾）、`:4247`（续跑收尾）、`:4943`（run 结束）。run 被硬切时一个都没走到，整个 run 的对话轮次与工具结果**一条都不入 sidecar**；而工具副作用（文件写入）已经生效。这是"文件与对话分叉"的唯一来源。

**L3 恢复层（悬挂调用被静默剥离）**：`acp_server.rs:10597` 的测试名即契约——`dangling_tool_calls_from_sidecar_never_reach_the_provider`。加载时对"有 tool_call 无结果"的消息做静默剥离（为 provider 兼容），**连"上一轮有一个未回收的调用"这层事实都不留给下一轮**。模型因此不知道自己发起过一个没回来的命令，只会照着旧现场重做。

三层叠加的净效果：**用户看到"没有任何反应"，模型看到"我停在 §10"，磁盘上是"§11 已完成"。** 三者互不一致。

## §3 同型复现

| run | 终止前最后三事件 | 时刻（UTC） | 终态事件 |
|---|---|---|---|
| RUN-10 | `permission_decision` → `control_ticket_*` → `tool_started` | 04:46:11 | 无 |
| RUN-11 | 同上（`call_00_WokxxG8vgrbOzQhNePSO9268`） | 04:55:00 | 无 |
| RUN-12 | 同上（`call_00_YUFNkP8C3cgBqbSQ3Qgb5056`） | 05:23:55 | 无 |

三次均无 `run_finished`／`run_cancelled`／`run_invalidated`，即**没有一次走到终态路径**。同期正常 run（0/2/3/4/5/6/7/8/13）均有 `run_finished{status:completed}`，其中 7、8 两轮 `terminal_nudged=true`、`terminal_nudge_count=1`。

## §4 终态声明机制的失效模式（本会话自证）

1. **格式脆弱**：本批执行中，助手把声明写成 `<RUN_END>`（尖括号），机械层不识别 → 触发 `[TERMINAL_NOTICE]` → 多一次往返。同一份语义，两种括号，结果完全不同。
2. **完整答复被降级为草稿**：run 11 的 04:52:50 文本是一次完整汇报，因未带声明而未交付、未落盘；loop 继续调用模型，随后 run 死在工具里——**这一轮唯一一次面向用户的完整表述彻底消失**。
3. **催收制造收尾偏置**：常驻句「结束前输出 [RUN_END]」（`prompt.rs:59`）＋窗口尾【结束自述通道】＋`[TERMINAL_NOTICE]`（`prompt.rs:69`，`TERMINAL_NUDGE_MAX=2`，`agent_loop.rs:2219`）三处同向，每轮都在把"继续"翻译成"收尾"。这是"急着完成任务"观感的第一来源（RLI 是第二，见 §5.3）。
4. **语义不可靠**：机制假定模型知道自己是否"完成"。run 11 的模型自认完成并写出答复，实际还有一个未执行的 §12——**模型对"完成"的判断本来就是它最不可靠的判断之一**，把它做成必填字段，等于把不可靠判断固化成账。**

## §5 设计判决建议

### 5.1 终态：默认零声明，声明降为不阻断元数据

- **回合边界由结构决定**：无 `tool_calls` 的文本＝回合结束，**立即交付、立即落盘**（`terminal_source=text`）。不再要求任何标记。
- **声明改为可选元数据**：若模型想表达"未完/阻塞/等外部"，在正文里写普通段落（如 `## 状态` 三行）。机械层能解析就落账，解析失败记空——**永不因此扣留答复**。
- 理由：文本契约的失败是静默的、代价是丢一轮；结构事实的失败是可见的（没输出就是没输出）。

### 5.2 机器可读终态只在需要它的车道，且用工具而非文本

- benchmark／headless／ACP 外部客户端确实需要机器可读终态：改为**一次收尾工具调用** `finish(status, summary)`——schema 可校验、幂等、不会因方括号写错失效、不占正文。
- `ORZ_TERMINAL_DECLARATION=required` 只对这些车道生效；交互式 Web/TUI 缺省 `off`。
- **删除 `[TERMINAL_NOTICE]` 催收**，或只保留"输出为空/复读"的真退化触发（那是哨兵，不是契约）。

### 5.3 RLI：推送面删除，只留拉取面（用户 2026-10-25 口述方向）

- 现状：工具结果尾部注入（本会话 16 次），文案含 `T̂(秒)`／`前推(10·T̂)`／`stuck`／`low_progress`。拉取面已存在（`blackboard_read section=rli`），推送是重复建设。
- 代价：占窗口；把"读数"污染成"评价"；**唯一带秒单位的信号**与"无墙钟"设计意图直接冲突。
- 建议：① 删尾部注入；② guide 加一句"需要自评节奏时读 `section=rli`"；③ 被动面只留退化检测一条，纯事实、无时间单位、无域标签；④ 观测值进 `mechanical_audit`／`run_finished`，模型不可见。
- 代价交代：纯拉取后模型基本不会主动读＝RLI 事实上退役。若其本意是"给模型当特征"，"不读"就是结论；要保留被动能力，只留③那一根针。

## §6 处置清单（建议，未实施）

| 级 | 项 | 落点 | 验收 |
|---|---|---|---|
| P0-1 | 回合级写前落盘（assistant 轮含 tool_calls 先写 sidecar，结果回写一次） | `acp_server.rs` 持久化点 | 强杀 run 后 sidecar 保留至少到最后一个 `tool_completed` |
| P0-2 | 悬挂调用不静默剥离：窗口头注入「上轮中断，未回收 `tool(call_id)`，结果未知、落盘副作用可能已发生」 | 加载路径＋`prompt.rs` 注入块 | 断 run 后下一轮首帧可见该事实 |
| P0-3 | 单工具无输出超时（非 run 墙钟）：`tool_started` 后 N 秒无 stdout/心跳 → 补 `tool_completed(error)` 并交回模型 | 工具执行器 | 挂死命令在 N 秒内可见失败，不再吞整轮 |
| P1-1 | 交付与收束解耦：无工具调用的文本无条件交付并落盘 | `agent_loop.rs:4254-4301` | 不带 `[RUN_END]` 的完整答复也到达用户 |
| P1-2 | 终态声明降级：交互车道缺省 `terminal_declaration=off`；benchmark 车道改 `finish()` 工具 | `model_stop.rs`／CLI 旗 | 交互轮次零 `[TERMINAL_NOTICE]` |
| P2-1 | RLI 推送面删除（§5.3） | `controller.rs` 拼装点 | 工具结果尾部不再出现 `RLI提醒` |

**本批实施状态（2026-10-25，用户裁决"按你的想法来、不必考虑修改成本"）**：

- 已落码：P1-1（终态声明缺省关闭；系统提示"结束前输出"仅在 `required` 车道拼接；常驻尾行在非 required 车道返回空串并跳过装配）、P1-2 的交互侧（`ORZ_TERMINAL_DECLARATION=required|on|1|true|yes` 显式开启，benchmark 车道行为不变；`finish()` 工具**未做**——八工具面冻结，暂以 env 开关承载）、P2-1（RLI 通知只落 journal，不再拼进工具结果）、时间基准统一（§9.1）。
- 未实施：P0-1/P0-2/P0-3（host 侧回合级落盘与悬挂调用对账），需下一批专门处理——它们是本事故"上下文回退"的直接修复面。

## §7 未决与待裁决

1. **本批索引登记**：`353` 是否为正式批号、是否立项（当前按"审查件、计数不变"处理），待用户裁决。
2. **工具挂死的直接触发**：三次命令体均为长 heredoc，但未取得"命令卡在 stdin"的直接证据（无 stdout 残留）；若要钉死，需在工具层加"命令体回显＋超时快照"。
3. **`dangling tool_calls` 的处理口径**：静默剥离是 provider 兼容性需要，改成注入事实需要先确认各家 provider 对"悬挂 tool_call 后接 user 消息"的容忍度。
4. **RLI 退役后的替代**：若模型确实需要"节奏"特征，应改为 `blackboard_read section=rli` 的按需读，还是并入 `context_manage` 回执，待裁决。

## §8 证据清单

- 会话侧车：`.gsa/conversations/51da03ea.json`（324 条 / 331,809 字符；末条 assistant＝"R9 §10 / 13022 字符 / 303 行"）。
- run 事件流：`.gsa/runs/RUN-51da03ea-{10,11,12,13}/events.jsonl`（10/11/12 无终态；13 的 `face_fingerprint`＝114 条 / 121,058 字符）。
- 磁盘对照：`BRANCH_DISCUSSION_LITERATURE_R9_2026-10-22.md`（16,501 字符、`## 11` 四小节、mtime `2026-10-25 12:52:39`）。
- 缺失对照：侧车全文检索 `16499`／`断线前那次补跑`／`队列收尾` 均 0 命中。
- 源码：`orz-assurance/src/journal/recorder.rs:230-232`（时间戳＝`Utc::now()`）；`orz-host/src/acp_server.rs:3642/4247/4943`（侧车落盘点）、`:10597`（悬挂调用剥离测试）；`orz-loop/src/prompt.rs:59/69`；`orz-loop/src/agent_loop.rs:2219/2230/4254-4301`；`orz-loop/src/model_stop.rs`（声明解析）。
- 时间基准说明（历史件）：`events.jsonl` 存的是**本机时钟的 UTC 序列化**（`+00:00`），本地时间＝UTC+8；R9 文件 mtime 12:52:39 `tool_completed` 04:52:39Z 逐秒吻合。353 起新落盘统一为北京时间 `+08:00`（§9.1），历史件不回写。

## §9 实施记录（本批落码，未提交）

### §9.1 时间基准统一为北京时间

- 新增 `orz-assurance/src/clock.rs`：`beijing()`（固定 +08:00，不依赖 `TZ`）、`now()`、`now_opts(SecondsFormat)`、`to_beijing_opts(DateTime<Utc>, …)`；`lib.rs` 增 `pub mod clock;`。
- 21 处生产者改口径：`journal/recorder.rs`×1、`plan/state_machine.rs`×3、`plan/artifact.rs`×1、`credential/mod.rs`×1、`permit/mod.rs`×1、`acaf/mod.rs`×1、`orz-loop/controller.rs`×1、`orz-loop/retrieval/evidence.rs`×4、`orz-host/session.rs`×2、`orz-host/acp_server.rs`×3、`orz-host/keystore.rs`×2、`orz-bin/main.rs`×1。
- 安全性：同一瞬时的 `Z` 串与 `+08:00` 串解析后相等（`clock` 单测 `same_instant_z_and_beijing_compare_equal` 钉死）；安全材料比较面一律 `DateTime::parse_from_rfc3339`（`acaf::predates_restart_cutoff`），不受偏移影响。

### §9.2 终态声明（P1-1／P1-2 交互侧）

- `model_stop::terminal_declaration_required()` 缺省 **false**；`ORZ_TERMINAL_DECLARATION=required|on|1|true|yes` 开启。
- `declaration_required_for()` 由 **AND** 改 **OR**：controller 车道位与 env 各自可独立开启（原 AND 在 env 缺省关闭时会把 benchmark 车道一起拖死）。
- 系统提示里的「结束前输出 [RUN_END]（intent/reason/summary）」仅在 required 车道拼接。
- `model_stop_resident_line()` 在非 required 车道返回空串；新增 `model_stop_resident_line_unconditional()` 供测试与 required 车道装配。`model_face` 两处装配改为空串跳过，`model_face_message_count` 同步（`count + 1 + channel`）。

### §9.3 RLI 推送面（P2-1）

- 工具批回传不再拼接 `RLI提醒:` 行（`agent_loop.rs`）；`mechanical_audit_update{kind: rli_notice}` journal 行保留（观测面不丢、模型不可见）。
- 拉取面 `blackboard_read section=rli` 不变。

### §9.4 验证

- `cargo check -p orz-loop -p orz-host -p orz-bin`：Finished，0 error。
- `cargo test -p orz-loop --lib`：**908 passed / 0 failed / 3 ignored**。
- `cargo test -p orz-host --lib -- --test-threads=1`：**436 passed / 0 failed / 6 ignored**（与 350 批基线同形）；默认 8 线程下 5 例进程树/超时用例 flaky（`run_tests_timeout_kills_process_tree`、`call_tool_timeout_kills_process_tree`、`call_tool_with_timeout_override_is_honored`、`session_volume_symlink_windows_end_to_end`、`run_terminal_cmd_truncation_carries_output_object`），串行全绿——判定为并行负载下的既有 flaky，非本批改动引入。
- 未跑：`orz-bin` 套件与 `frontend_smoke.mjs`（本批未触及 CLI 旗面与前端）。

### §9.5 P0-2 补批（2026-10-25 同日，已落码）

- **P0-2 悬挂调用不再静默剥离**：`conversation_repair::repair_tool_pairing` 增 `RepairMode`——
  `Sidecar`（侧车存过工具输出，整条没回应＝真中断）保留声明并逐 call 补事实化结果
  `[上次 run 中断] 工具调用 <name>（call_id=…）没有返回结果…结果未知，落盘副作用可能已经发生`；
  `JournalRebuild`（journal 从不存工具输出）维持旧纪律剥离，避免把每段重建历史都说成失败。
  `acp_server.rs` 两处调用点按来源分流（`reloaded_from_journal` 判定）。`RepairReport` 增
  `unrecovered_calls`，与 `stripped_calls` 互斥。
- **口径变更（测试改判）**：`dangling_tool_calls_from_sidecar_never_reach_the_provider` →
  `…_reach_provider_as_failed_fact`（断言改为"声明与结果成对进入 provider"）；
  `session_load_materializes_archive_and_repairs_tool_pairing` 由"空声明被剥离（2 条）"改为
  "声明＋未回收结果成对保留（4 条）"。
- 读数：`cargo test -p orz-host --lib -- --test-threads=1` **437 passed / 0 failed / 6 ignored**；
  `conversation_repair` 7 例全绿（新增 Sidecar 幂等用例）。

### §9.6 仍未实施（下一批）

- **P0-1／P0-3**：回合级写前落盘、单工具无输出超时——本事故"上下文回退"的直接修复面，本批未动 host 持久化路径。
  P0-3 取证（2026-10-25）：host 已有默认 300s 的 `tool_timeout`／`ORZ_TOOL_TIMEOUT_SECS`（`acp_server.rs:6184`、`lib.rs:458`、`lib.rs:1484-1487` 的 `tokio::time::timeout` 包裹），
  且同会话 `RUN-0` 存在多条 `tool_completed{status:error}`（机制本身会落账）。复核 `RUN-51da03ea-11/12` 事件流：末事件均为 `tool_started`（04:55:00.828Z／05:23:55.498Z），
  **此后到下一次 run 之间没有任何收尾事件**——即 300s 预算并未走到（否则 04:59:58 应有 `tool_completed`）。结论：进程在预算内终止，**进程内看门狗救不了这类事故**；
  唯一有效面是 P0-1（执行前落盘）＋P0-2（下一轮对账，已落），P0-3 只剩"车道是否同一 wrapper"的剩余核查。
- `finish()` 收尾工具：八工具面冻结，暂由 `ORZ_TERMINAL_DECLARATION` 承载机器可读终态。

