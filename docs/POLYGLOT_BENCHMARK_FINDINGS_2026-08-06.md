# Polyglot 摸底问题记录（2026-08-06）

**日期**: 2026-08-06
**背景**: Aider Polyglot 语义摸底（exercism Python difficulty 1-3，92 题，orz `--real --allow-write` 后端）。本文件记录测试暴露的**全部工具链与客观架构实现问题**（现象 / 根因 / 证据 / 状态 / 处理），供系统性修复。

**最终成绩（可信口径）**: 92 题基线 78/92（84.8%）；thinking 修复（P2）后的补充轮次再过 4 题（connect / resistor-color-expert / saddle-points / wordy）→ **82/92（89.1%）**。**最终稳定失败 4 题**（error-handling / tree-building / state-of-tic-tac-toe / transpose）——全部为**无测试反馈环下的盲改**（P6：模型实现大体正确但无法自验证迭代，撞轮次上限）。重跑循环已按用户指示终止（2026-08-06）。

---

## 问题清单

### P1. 权限 deny 的工具调用不回放 Tool 消息（协议断裂）——已修复

- **现象**: 模型调用被拒工具（如 search_replace 权限 deny）后，下一轮请求报 `400 invalid_request_error: An assistant message with 'tool_calls' must be followed by tool messages responding to each 'tool_call_id'`，整轮 run 以 `run_failed` 终止。
- **根因**: `run_host_tool` 的 deny 分支（controller.rs）直接返回 `Ok(ToolResult)`，**在 Tool 消息回放之前**——assistant 的 tool_calls 声明无配对应答。成功路径（1118 行）与 retrieval 路径（943 行）都回放，唯独 deny 路径漏。
- **证据**: 写探针 journal——`permission_decision deny → run_failed(400)`，无 tool 消息；真实 API 返回 `insufficient tool messages following tool_calls message`。
- **处理**: 已修复（orz `3e32ed7`）——deny 分支与成功路径一致 push Tool 消息（content="denied by permission gate"，tool_call_id 配对）+ 回归测试 `denied_tool_round_replays_tool_message`（journal 无 tool_started 的 deny 证据纪律不变，协议层完整应答）。
- **教训**: 协议完整性（每个声明必有应答）是硬约束，任何提前 return 的路径都必须审计回放。

### P2. DeepSeek V4 thinking 默认开启 → content 恒空 → 模型「话全丢」——已修复（重大）

- **现象**: 复杂任务（connect/transpose/resistor-color-expert 等）模型**零文本**：每轮 `model_output` text 为空，只调 read/list 工具，最终空轮结束——既不写文件也不输出答案。
- **根因**: V4 **thinking 模式默认开启**（官方文档确认）。模型把全部 token 预算烧在 `reasoning_content`，`content` 从未生成。orz 只消费 content（文本流 + tool_calls）→ **模型的话全在 reasoning 里，orz 听不见**。IP1 原实现只是「请求不传 thinking 参数」——V4 默认开 = IP1 名存实亡。
- **证据**（原始 API 探测）:
  - 4096 预算: `content: ''` + reasoning 15883 字符 + `finish_reason: length`
  - 16384 预算: 201 秒 + reasoning 57413 字符**不收敛** + content 0 + length
  - `thinking: {"type": "disabled"}`: 9.8 秒 + content 2497 字符完整实现 + `finish: stop`
- **处理**: 已修复（orz `32b319d`）——fork（vendored）加 `ThinkingConfig` + `CreateChatCompletionRequest.thinking` 字段；transport `build_request` 显式 `thinking: {"type": "disabled"}`。IP1 测试语义更新（`thinking.type==disabled` + reasoning_effort 恒 None）。跑分实证：connect 从「零文本不写」→ 第一轮通过。
- **推论**: thinking 开启时工具轮**必须回放 reasoning_content**（官方文档）——这解释了 polyglot 中 phone-number 的偶发 400（见 P4）。disabled 后整个生命周期问题消失；回放机制（上轮提交）保留作防御。

### P3. 工具可用性三层断链：模型看不到「当前策略下哪些工具实际可用」——未修复（本审计新发现）

- **现象**: 模型反复尝试被拒工具（web_search/web_fetch 在 Benchmark 策略下被拒，模型一轮内连试 4-5 次），浪费轮次并撞 `MAX_TOOL_ROUNDS=8` 上限（resistor-color-expert 8 轮中 4 轮是 web 工具；state-of-tic-tac-toe 两轮都 web 轰炸）。
- **根因**: 三层信息全部与实际权限策略脱节:
  1. **tools 声明**: `tool_defs.clone()` 全量 28 工具静态传给模型（controller.rs:414）——被策略拒的工具也声明为可用；
  2. **availability block**（IP2a）: headless 映射「every advertised tool is available」（controller.rs:295-296 恒 `Some(true)`）——与权限策略零关联；
  3. **deny 回传**: 仅被动单次（「denied by permission gate」），无「此工具在当前策略下不可用、请勿重试」的系统性告知。
- **证据**: journal 工具序列 `['read_file', 'list_dir', 'read_file', 'web_search', 'web_fetch', 'web_fetch', 'web_fetch', 'web_search']` + `gate_decision: tool_rounds_limit(8)`。
- **处理（建议）**: ① tools 声明按策略过滤（Benchmark 下不声明 web/bash/run_terminal_cmd——模型根本不会尝试）；② availability block 反映策略（headless 不再恒全 true）；③ deny 消息强化为「不可用」语义（含工具名 + 策略名）。**这是「框架所有内容被模型完全掌控」的核心闭环**。

### P4. 偶发 400（thinking 开启时 reasoning_content 回放不完整）——随 P2 消除

- **现象**: phone-number 第二轮偶发 `400 insufficient tool messages`（与 P1 同错误但非 deny 路径）。
- **根因**: thinking 开启时，assistant 消息的 reasoning_content 在多轮中偶发不完整回放（或与 tool_calls 声明的组合在特定序列下触发 API 校验）——官方文档：「thinking enabled + tool calls 必须回放 reasoning_content 否则 400」。单跑复现不了（偶发、并发下出现），机制上与 P2 同源。
- **处理**: P2（thinking disabled）消除整个生命周期；回放机制保留（防御未来 thinking 模型）。

### P5. harness 假 PASS（测试执行方式错误）——harness 侧已修

- **现象**: 44 题结果不可信——被拒工具题（binary/octal 等）「失败」实为 harness 执行错误；leap 等「PASS」实为假 PASS。
- **根因**: harness 用 `[sys.executable, test_file, pytest_flags]` 直接执行测试脚本。exercism 测试文件两种风格：① 末尾 `unittest.main()`——拒绝 pytest 参数（`unrecognized arguments: --no-header`）；② 无 `__main__` 块——直接执行**零测试**且 exit 0（假 PASS）。
- **处理**: 改用 `python -m pytest` 归一化（两风格统一走 pytest）。修复后全量重跑（92 题可信基线）。

### P6. 无测试反馈环：模型盲改 + 找隐藏测试浪费轮次——harness 缓解已生效，系统级待裁决（**最终 4 稳定失败全部归因于此**）

- **现象**: 第二轮模型明确尝试读测试文件（transpose:「Let me read the test file」、state-of-tic-tac-toe:「The test file isn't present」）——隐藏测试在任务目录外（polyglot 语义），模型找不到后盲改，撞轮次上限。
- **根因**: 自主 agent（自己改文件）没有外部测试反馈环（aider 是对话式：模型出 diff → aider 跑测试 → 反馈）。orz 单 run 内模型无法自验证。
- **处理**: harness 第二轮 prompt 加「测试文件不可见，勿寻找，仅依据错误消息」声明（缓解——saddle-points/wordy 第二轮借此通过）。系统性问题：自主 agent 的验证闭环（工具内自测或 harness 级多轮）属 beta 测设计决策，待裁决。**终态确认**：4 稳定失败（error-handling/tree-building/state-of-tic-tac-toe/transpose）第二轮模型均完成合理实现但无法验证——非协议/架构 bug，是反馈环缺失的能力边界。

### P7. MAX_TOOL_ROUNDS=8 对分析型任务偏紧——未处理

- **现象**: 分析型任务（读 3-4 文件 + 多轮 search_replace + 验证）多题撞 8 轮上限（gate_decision tool_rounds_limit 出现于 transpose/wordy/error-handling/tree-building 等）。
- **根因**: `MAX_TOOL_ROUNDS: u32 = 8`（controller.rs:53）——Python 参考实现的口径移植，对 V4 非 thinking 模式的工具密集工作流偏紧。
- **处理（建议）**: 参数化（配置/CLI 可调）；或默认提高（8→16），保留反 runaway 语义。

### P8. 非流式 generate 无超时（大预算挂死风险）——未处理

- **现象**: 16384 预算探测 201 秒不收敛；orz `-p` 走非流式 generate（fork reqwest 零超时，Slice #11 记录）——大 max_tokens 下可长时间阻塞。
- **根因**: 非流式 create 路径无超时；流式路径有 select! 取消但 `-p` 不用流式。
- **处理（建议）**: 非流式加超时（或 `-p` 也走 generate_stream）；与 P7 联动（预算上限约束）。

### P9. reasoning_content 输出侧丢弃（防御性记录）——已部分处理

- **现象/根因**: 模型生成的 reasoning_content 不进 journal text、不给用户看（设计意图：TUI 展示答案不展示思维链）。thinking disabled 后实际不再产生——本项降级为防御性记录。
- **处理**: 回放（输入侧）已实现（上轮提交）；输出侧按设计丢弃，thinking disabled 后无实际影响。若未来启用 thinking 模型，需重新裁决「思维链是否入 journal」。

---

## 系统性问题总结（用户关注点）

**「极端条件下工具调用不成体系」的真相**:
1. **协议层**（P1/P4）: 部分路径（deny）漏回放 → 链条断 → 400。已修 + 审计教训。
2. **信息层**（P3/P6）: 模型对「可用工具集 / 当前策略 / 环境限制」的认知与实际脱节——tools 声明静态、availability 恒全 true、deny 被动、测试不可见。**这是主缺口**：模型在「以为可用」的工具上空转，动作因工具处无后续信息而断掉。
3. **迭代层**（P6/P7）: 无外部反馈环 + 轮次上限 → 盲改中断。

**「不做心跳或主动探针，框架所有内容需被模型完全掌控」的落点**（P3 为核心）:
- 工具声明 = 策略过滤后的真实可用集（模型永不尝试被拒工具）
- availability block = 运行时状态（权限策略 + 环境限制）
- deny 消息 = 不可用语义（一次性告知，含策略名）
- 环境约束（无网络等）= system prompt 前置声明

## 状态一览

| # | 问题 | 状态 | 落点 |
|---|---|---|---|
| P1 | deny 回放缺失 | ✅ 已修复 `3e32ed7` | orz-loop controller.rs |
| P2 | V4 thinking 默认开启 content 恒空 | ✅ 已修复 `32b319d` | fork + transport |
| P3 | 工具可用性三层断链 | ⬜ 未修复（建议方案已列） | controller + permission |
| P4 | 偶发 400（thinking 回放） | ✅ 随 P2 消除 | — |
| P5 | harness 假 PASS | ✅ harness 侧修复 | run_eval.py |
| P6 | 无测试反馈环 | ⬜ harness 缓解，系统级待裁决 | harness + 设计 |
| P7 | MAX_TOOL_ROUNDS=8 偏紧 | ⬜ 未处理（建议参数化） | controller |
| P8 | 非流式无超时 | ⬜ 未处理（建议加超时） | transport |
| P9 | reasoning_content 输出侧丢弃 | ✅ 防御性关闭（随 P2） | — |

## 入口

- 跑分: `%TEMP%\polyglot-eval\run_eval.py`（harness）+ `results.jsonl` + `tasks/`（失败保留目录 + journal）
- 修复: orz `3e32ed7`（deny 回放 + Benchmark）/ `32b319d`（thinking disabled）
- 本文件: `docs/POLYGLOT_BENCHMARK_FINDINGS_2026-08-06.md`
