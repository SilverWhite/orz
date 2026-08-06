# reasoning_content 回放缺口闭合（alpha 测发现 + fork 补丁）

**日期**: 2026-08-06
**状态**: 代码完成并验证，fork push 待用户手动
**范围**: async-openai fork（our-forks）+ orz-loop 传输层 + 回放闭环

---

## 1. 发现（alpha 测 live 验证）

Slice #11 设计审查曾记录「reasoning_content 回放缺口——V4 thinking 模型可能要求，fork 无字段，live 验证后打 fork 补丁」。alpha 测（2026-08-06）live 验证用原始 API 探测实证：

- **非流式响应**：`message` 含 `reasoning_content` 字段，实测 318 字符真实推理内容（**即使未开 thinking 选项**——IP1 thinking:disabled 仍返回）
- **流式响应**：每个 delta 块携带 `reasoning_content`（与 content 交错）
- fork（95b52eb）三个消息类型均无此字段 → serde 静默丢弃 → **每轮推理内容全部丢失**，多轮上下文不完整（工具轮因当前模型未强制校验而侥幸成功，非协议正确）

缺口从「可能」升级为「实际存在」。

## 2. 用户裁决

`--real` 显式 flag 接线（alpha 测第②步）+ reasoning_content 补丁落地方式：**本地改 fork → orz 侧闭环 → 全量验证 → 验证后一并 push**（fork + orz 主仓，rev 最后更新）。

## 3. fork 改动（commit `bf46d7a`，分支 `reasoning-content-replay`）

`async-openai/src/types/chat/chat_.rs` 三处，+16 行：

1. **`ChatCompletionRequestAssistantMessage`**：`reasoning_content: Option<String>`（`skip_serializing_if`——None 不上线，OpenAI/xAI 等不产该字段的 provider 零影响）
2. **`ChatCompletionResponseMessage`**：`reasoning_content: Option<String>`（非流式响应保留，供传输层提取）
3. **`ChatCompletionStreamResponseDelta`**：`reasoning_content: Option<String>`（流式分块累积）

模式与既有字段完全一致（`skip_serializing_if = "Option::is_none"`），零破坏。

## 4. orz 侧闭环

- **`model.rs`**：`Message` + `ModelResponse` 加 `reasoning_content: Option<String>`（注释言明仅 Assistant 消息设置）；17 处字面量构造点补齐
- **`transport.rs`**：
  - `map_message` assistant 分支——声明消息带 `reasoning_content` 回放（**回放闭环核心**）
  - `from_response`——非流式响应提取（absent-optional，非空串）
  - `generate_stream`——`reasoning_parts` 独立累积（**推理 delta 永不作 live text delta**——TUI 展示答案不展示思维链），逐字拼接
- **`controller.rs`**：工具轮 assistant 声明消息（Slice #11 D2-1 回放点）带上 `response.reasoning_content`
- **`fake.rs`**：`ScriptedResponse` + `with_reasoning()`（测试可脚本化回放路径）
- **IP1 语义澄清**：`build_request_omits_thinking_ip1` 断言从 `!contains("reasoning")` 收紧为 `!contains("thinking")` + `!contains("reasoning_effort")` + `!contains("reasoning_content")`（request() 携带 None 时）——**reasoning_content 是内容回放字段（模型已产出的内容），非推理开关**；IP1「请求面永不启用 thinking 选项」不变

## 5. 测试（+3，71→74）

| 测试 | 锁定 |
|---|---|
| `build_request_replays_assistant_reasoning_content` | assistant 消息回放序列化 |
| `from_response_extracts_reasoning_content` | 非流式提取 + absent-optional |
| `tool_round_replays_reasoning_content_on_declaration` | controller 工具轮声明带 reasoning（fake with_reasoning，received[1] 断言） |

`stream_aggregation_concatenates_chunks_and_tool_calls` 扩展：reasoning delta 交错累积断言 + 永不作 live delta。

## 6. 验证

- orz-loop **74 passed / 0 failed**（+1 ignored live）；clippy 零警告；fmt 零漂移
- 兄弟回归：orz-host 78 / orz-tui 174 / orz-bin 3 / orz-codex 34 全绿
- **真实冒烟**（`--real -p` 工具轮）：带 reasoning_content 回放的请求被真实 DeepSeek 接受（**无 400**）；journal 15 事件完整链（preflight→…→tool_started→tool_completed→双 model_output→counterexample_gate→run_finished）过 Python 交叉验证器 **0 errors**

## 7. 待办（推送用户手动惯例）

1. `git -C %TEMP%\async-openai-fork push origin reasoning-content-replay`（fork push；403 因当前凭据无 our-forks 写权限）
2. orz `Cargo.toml` rev `95b52eb…` → `bf46d7aac8d873a327e6472c6beb6f7925a5eda0`，`cargo fetch` + lock 更新 + 复测后提交（代码与 rev 原子提交——当前代码已在 dirty checkout 下验证）
3. orz 提交 → `cli/feat/fusion-architecture`，主仓库审计文档/索引/README 收尾 → `origin/main`

## 8. 入口

- fork: `%TEMP%\async-openai-fork` / `B:\.cargo\git\checkouts\async-openai-80cbf7d6f25979e0\95b52eb\async-openai\src\types\chat\chat_.rs`（dirty 验证态）
- orz: `D:\CLI\orz\crates\orz-loop\src\gateway\{model,transport,fake}.rs` / `crates\orz-loop\src\controller.rs`
- 本文件: `docs/REASONING_CONTENT_REPLAY_SLICE_2026-08-06.md`
