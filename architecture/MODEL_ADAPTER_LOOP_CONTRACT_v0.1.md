# Provider-neutral Model Adapter Loop 契约 v0.1

状态：development-only、mock-transport-only implementation spike；不修改 protocol v0.1 的 gate、action、evidence 或 claim 语义。

检查基线：2026-07-21 DeepSeek 官方 V4 OpenAI Chat Completions 文档。

## 1. 分层

```text
model loop
  -> provider adapter (DeepSeek request/SSE/transcript rules)
    -> StreamingTransport protocol
      -> ScriptedTransport (本阶段唯一实现，无 socket、无 API key)
```

adapter 不拥有权限裁决或工具执行；transport 不理解 session、journal 或 reasoning continuity。model loop 负责有限重试、journal 顺序、固定 mock tool allowlist 和 terminal run event。

## 2. Request 不变量

- profile 先通过 `deepseek-adapter-profile-v0.1` 和语义 preflight；
- V0 只实现 OpenAI Chat Completions body，Anthropic compatibility 继续 fail-fast；
- `stream=true` 且 `stream_options.include_usage=true`；
- thinking mode 不发送 sampling 字段；
- tools/profile capability 必须一致；
- 每次请求前验证 tool-call ID、strict JSON arguments 和 thinking `reasoning_content` continuity；
- Authorization/API key 不属于 request body，也不由 adapter 接受；
- journal 只保存 request digest、message roles/count 和 continuity boolean，不保存 message content。

## 3. SSE 不变量

- 空行是 separator，`: keep-alive` 只计数；两者都不是语义输出；
- `data: [DONE]` 必须出现；缺失时 stream 无效；
- usage-only chunk 可使用 `choices=[]`；
- V0 只接受 choice index 0；
- content、reasoning 和 tool-call arguments 支持跨 chunk 拼接；
- tool-call index 必须从 0 连续，最终 arguments 必须是 strict JSON object；
- `tool_calls` finish reason 必须确有工具调用；`stop` 必须有非空 content；
- `length`、`content_filter`、`insufficient_system_resource` 保留为非成功状态，不伪装成完整回答。

## 4. Provider-private reasoning

raw `reasoning_content` 只允许存在于当前进程内存：

1. 从第一轮 SSE 组装 assistant message；
2. thinking tool call 后完整回传到下一请求；
3. 普通 journal/result 仅记录 present、UTF-8 byte count 和 SHA-256；
4. mock output root verifier 拒绝出现 `reasoning_content` 字段名；
5. reasoning digest 不是 ReasoningPrecommitment，也不是 evidence。

## 5. Mock tool loop

CLI smoke 固定为两次模型请求和一次 `mock_echo`：

- scripted response 1 产生 thinking + `mock_echo({"value": 42})`；
- 本地固定 allowlist 执行器验证 name/arguments 后返回确定性 JSON；
- scripted response 2 检查历史 continuity 后产生最终 JSON content；
- 任何其他工具名或参数形状直接失败；CLI 不接受自定义 tool 或 response script。

该工具执行是 adapter-loop contract fixture，不是通用 ToolBroker，也不产生 ArtifactRecord/EvidenceRecord。

## 6. 重试与 terminal

- 只对冻结 profile 中的 429/500/503 做有限次数重试；
- 每一次 HTTP attempt 都分别 journal `model_request` 与 `model_output`；
- 400/401/402/422 或未知状态不自动重试；
- retry 不得改变 model、messages、tools 或 request digest；
- 成功 loop 恰有一个 `run_finished`；结构或 transport 失败尽力写一个 `run_failed`，并留下 `_INCOMPLETE.json`；
- request/HTTP 成功不等于科学任务或 claim 成功。

## 7. 已接受限制

- 没有外部 HTTP、TLS、DNS、proxy 或真实 provider transport；
- 没有首事件/总 wall-clock timer 的 socket-level 实现；
- private transcript 没有持久化，因此尚不支持崩溃后恢复 thinking tool turn；
- journal 仍是单 writer、无多进程锁；
- mock permission decision 不是交互授权；
- run-manifest v0.1 强制存在 temperature 字段，thinking smoke 将其标为未发送的 schema compatibility placeholder。

补充实现：`LOOPBACK_TRANSPORT_AND_PRIVATE_TRANSCRIPT_v0.1.md` 现已在不开放外部 endpoint 的前提下实现真实本地 HTTP framing、cooperative cancellation/timeout，以及 Windows current-user DPAPI private transcript。HTTPS、认证和真实 DeepSeek 仍不属于本契约。

第二个补充层：`DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md` 已实现固定 endpoint 的 HTTPS transport 构造、一次性 request-digest permit 与 Windows Credential Manager provider，但只通过 injected fake connection 验证，尚未接入本 model loop，也没有可签发许可的联网 CLI。

第三个补充层：`NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md` 已把 turn/attempt/retry context 和 redacted confirmation summary 接入 model loop。内建 in-process fake provider 会经历 503 后重试及第二轮 reasoning continuity，共消费三个独立 permit；任意真实 connection factory 仍因 `real_network=true` 被 mock loop 拒绝。

## 8. 官方来源

- [Create Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion/)
- [Thinking Mode](https://api-docs.deepseek.com/guides/thinking_mode/)
- [Tool Calls](https://api-docs.deepseek.com/guides/tool_calls/)
- [Rate Limit & Isolation](https://api-docs.deepseek.com/quick_start/rate_limit/)
- [Error Codes](https://api-docs.deepseek.com/quick_start/error_codes/)
