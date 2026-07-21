# Per-attempt Network Permit Broker 契约 v0.1

状态：development-only、in-process-fake integration spike；不签发真实外网权限，不读取真实凭据，不修改 protocol v0.1 语义。

## 1. 为什么 permit 必须按 attempt 签发

一个 model turn 可能因 429/500/503 产生多个实际 HTTP 请求。如果只按 turn 批准，有限 retry 会把一次授权扩张成多次计费和多次数据披露。因此 transport attempt context 明确记录：

- `turn`；
- `attempt`；
- retry 前一次 HTTP status。

第一次 attempt 不允许携带 previous status；第二次及以后必须携带。每次调用 broker 都生成新的 one-shot permit，先前 permit 即使请求 body digest 相同也不能复用。

## 2. 可审计确认摘要

`network-confirmation-summary-v0.1` 保存以下事实：

- 固定 provider/endpoint、model、turn/attempt/retry provenance；
- strict JSON body byte count 和 SHA-256；
- message role 序列及消息、tool definition、tool result 数量；
- 是否将发送 message content、tool definitions、provider-private reasoning；
- stream、response format、max tokens；
- connect/first-semantic/total timeout 与 request/response byte limit。

摘要不得包含 message content、tool arguments、raw `reasoning_content` 或 Authorization。摘要本身再以 RFC 8785 canonical JSON 计算 SHA-256，transport metadata 同时记录摘要和摘要 digest。

“将发送私有 reasoning=true”是披露提示，不是 reasoning 内容，也不是 EvidenceKernel 的 reasoning precommitment。

## 3. Broker 类型

- `DenyAllPermitBroker`：生产安全默认值，任何请求都拒绝；
- `ScriptedPermitBroker`：只用于确定性测试，按预期 turn/attempt 消费冻结 decision；
- `InteractivePermitBroker`：已实现 typed-summary-digest 交互，但被强制限制为内建 fake provider；详见 `INTERACTIVE_APPROVAL_LEDGER_CONTRACT_v0.1.md`。

Scripted broker 不能被包装成生产自动批准器。`BrokeredDeepSeekHttpsTransport` 在构造期强制它只能与内建 in-process fake provider 配对；默认真实 connection factory 会直接失败。它只证明每个 attempt 都经过独立调用、摘要与 permit 绑定正确。

## 4. In-process fake HTTPS provider

`InProcessFakeDeepSeekConnectionFactory` 是无 socket fixture：

- 只接受固定 `api.deepseek.com:443` 与安全 TLS context；
- 验证 POST path、Bearer header 存在及 strict JSON body；
- 不记录 Authorization 值或消息正文；
- 可在内存 validator 中确认第二轮 reasoning continuity，随后只留下计数与 digest；
- 预置 503、第一轮 tool-call SSE、第二轮 final SSE 三个 exchange。

该 factory 具有明确 `is_in_process_fake_provider=true` marker；只有这一内建 fixture 会使 brokered transport 暴露 `real_network=false`。任意普通 injected factory 仍按 external-capable 路径处理，不能靠调用者布尔参数伪装成无网络。

## 5. 两轮集成 smoke

`deepseek-brokered-fake-https-smoke` 执行：

1. turn 1 attempt 1：permit #1，fake HTTP 503；
2. turn 1 attempt 2：permit #2，请求 digest 必须与 attempt 1 相同，完成 tool call；
3. 本地固定 `mock_echo`；
4. turn 2 attempt 1：permit #3，摘要声明会回传一条 provider-private reasoning message；
5. 完成 final JSON，并沿用 journal、DPAPI transcript 和 verifier。

结果中保留三份 redacted confirmation summary。普通文件扫描必须找不到 fake API key、raw private reasoning 或 `reasoning_content` 字段。

## 6. 尚未实现

- 已有 fake-only 终端确认 UI 与 hash-chained approval ledger，但没有可授权真实网络的 broker；
- 没有真实 Credential Manager 读取；
- 没有 DNS、socket、TLS handshake、DeepSeek API 调用或计费；
- 没有把一次批准扩张为整个 session、整个 turn 或所有 retry；
- 没有评价模型质量、evidence 或 claim eligibility。

## 7. 相关官方来源

- [DeepSeek Create Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion/)
- [DeepSeek Rate Limit](https://api-docs.deepseek.com/quick_start/rate_limit/)
- [DeepSeek Error Codes](https://api-docs.deepseek.com/quick_start/error_codes/)
- [DeepSeek Thinking Mode](https://api-docs.deepseek.com/guides/thinking_mode/)
