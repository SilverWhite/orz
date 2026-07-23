# Grok ACP fake-tool continuity spike（2026-07-23）

状态：锁定 Grok `0.2.106` 的管理员级 Windows fake-only live smoke 已完成；allow/cancel 两个场景均经
独立 verifier 重验通过。未调用真实 Grok/DeepSeek 模型，未写真实用户 workspace，未提升任何 FEP/LIF
claim。

## 目标

本 spike 把已通过的 ACP `initialize` 探针推进到一个固定的 fake-only prompt turn，机械核验：

1. `initialize` → `session/new` → `session/prompt` 的请求顺序；
2. `tool_call` 与同一 `toolCallId` 的 `tool_call_update`；
3. `session/request_permission` 的 `allow_once` 响应；
4. permission pending 时发送 `session/cancel` 与 `cancelled` 响应；
5. 原始 ACP transcript、loopback provider capture、session `events.jsonl`/`updates.jsonl` 三路对账；
6. prompt response 恰好一个 terminal；allow 场景的工具 terminal 恰好一个；
7. restricted workspace receipt、临时 firewall 和 kill-on-close Job Object 不退化。

协议形状依据 ACP v1 的
[Prompt Turn](https://agentclientprotocol.com/protocol/v1/prompt-turn)、
[Tool Calls](https://agentclientprotocol.com/protocol/v1/tool-calls) 和
[Cancellation](https://agentclientprotocol.com/protocol/v1/cancellation)。锁定 Grok `0.2.106` 的本地
`15-agent-mode.md` 仍是 binary-specific 请求顺序依据；网站当前 v1 已描述 `$/cancel_request`，但本 binary
返回空 `sessionCapabilities`，实际路径仍是 `session/cancel`。本探针只登记观察到的锁定 binary 行为，不用网站
后续新增字段扩大 capability claim。

## 实现

- [`invoke_grok_acp_fake_tool_probe.ps1`](../scripts/invoke_grok_acp_fake_tool_probe.ps1)：
  管理员入口；在任何 Grok 进程前签发 restricted receipt，为已锁定 `grok.exe` 创建 non-loopback IPv4 与全
  IPv6 出站阻断，使用 clean environment、read-only sandbox 与 Job Object。
- [`run_grok_acp_fake_tool_client.py`](../scripts/run_grok_acp_fake_tool_client.py)：
  固定 ACP v1 client；只发送仓库内惰性 prompt，原始 transcript 仅写入被忽略的 run 目录，结果只投影 ID、
  method、status 和计数。
- [`fake_deepseek_provider.py`](../scripts/fake_deepseek_provider.py)：
  新增 `tool-cancel`；只返回第一次固定 `read_file` tool call，并以一次 primary request 为成功终态。
- [`verify_grok_acp_fake_tool_probe.py`](../scripts/verify_grok_acp_fake_tool_probe.py)：
  从 transcript、provider private capture、session files 和三张 trust receipt 独立重建投影，重算全部 artifact
  digest，并验证场景特定不变量。
- result 与 verification 分别受
  [`grok-acp-fake-tool-probe-result-v0.1.schema.json`](../integration/grok/grok-acp-fake-tool-probe-result-v0.1.schema.json)
  和
  [`grok-acp-fake-tool-probe-verification-v0.1.schema.json`](../integration/grok/grok-acp-fake-tool-probe-verification-v0.1.schema.json)
  约束。

## 两个场景

`allow_once` 要求：

- 只出现一个 `tool_call`；
- client 选择 agent 提供的 `allow_once` option ID；
- 同一 `toolCallId` 恰好一个 terminal `completed` update；
- prompt 以 `end_turn` 完成；
- provider 第二次请求保留 reasoning、tool call ID 和 tool result marker；
- session event 只记录一次 `tool_completed`。

`cancel_permission` 要求：

- permission request 到达后先发送 `session/cancel`，并以 `cancelled` outcome 回应 pending permission；
- prompt 以 `cancelled` 完成；
- 允许取消竞态表现为没有工具终态，或同一 tool ID 的唯一 `failed` 终态；两者都不得出现 `completed`；
- provider 只收到第一次 tool-call request；
- session event 不得出现 `tool_completed`。

## Windows observed 结果

最终证据保存在 git-ignored 的：

- `.observed-runs/acp-fake-tool-allow-live-v4`
- `.observed-runs/acp-fake-tool-cancel-live-v4`

两次均锁定 binary SHA-256
`a6a25d55daadca0c2458a5aceb4c1873eb7c76964ef307647d079e344c53969a`，`result.valid=true`，
内置与独立 verifier 均为 `valid=true`，且全机 `LIFGrokACPTool-*` 临时规则残留为 0：

| 场景 | probe ID | result SHA-256 | 关键观察 |
|---|---|---|---|
| allow | `ACPTOOL-17237ff23acd467cb11b15a0f933f9da` | `44d88ff3c8c4b1c38c12e3396fce23bf06533dd4055f233d91d2fc2813a6be40` | `allow_once`、`completed`、`end_turn`、2 primary requests、1 `tool_completed` |
| cancel | `ACPTOOL-e6e4c9637253415089a6ce0c30d9e5c8` | `8b3cd044cb6af4cf06d5103420227cad519371bdeacbbfdab591e31fbf10e855` | `cancelled`、无 completed terminal、1 primary request、0 `tool_completed` |

Grok 还各发出一次只用于 session title 的 loopback auxiliary request；verifier 限定其 model 为 `grok-4.5`、
唯一 tool 为 `session_title`、Authorization 只记录 presence 而不记录值。两次 `real_model_invoked=false`，
ACP `unexpected_message_count=0`。allow 的精确扩展通知计数为 `1/1/3/1/5/2`，cancel 为
`1/1/2/1/4/2`，依次对应 `_x.ai/mcp/servers_updated`、`_x.ai/mcp_initialized`、
`_x.ai/queue/changed`、`_x.ai/session/prompt_complete`、`_x.ai/session_notification` 和
`_x.ai/sessions/changed`。

## 调试链与修复

- 初次 live run 证明工具主链成立，但把六种精确的 `_x.ai/*` 通知和 session-title auxiliary request 错算为
  unexpected；现已逐 method/shape 白名单并由独立 verifier 重建。
- PowerShell 5.1 对空 `grok.stderr.log` 的 `Get-Content -Raw` 返回 `$null`；泄漏扫描现先归一化为空字符串。
- 取消实测同时观察到“无 terminal”与唯一 `failed` terminal；校验因此冻结真正的不变量——不得 completed、
  不得 `tool_completed`、不得第二次 primary request——而不伪造单一调度顺序。
- prompt response 与 session JSONL 落盘存在短竞态；客户端现等待期望 tool/permission 状态并稳定 250ms 后再
  投影，防止结果引用的文件在取证后继续变化。

静态与合成边界同时保留：Python 入口通过 compile，launcher 通过 PowerShell AST parse；verifier fixtures
覆盖 allow、cancel 两种取消终态、transcript/extension/provider 篡改；fake provider 继续覆盖
single/continuity/cancel/overwrite refusal。`fep-script-validation` 对三个 Python 入口仍为 0 FAIL、
0 catastrophic red line；experiment-domain WARN 不通过添加伪字段消除。

## 重复验收命令

从管理员 PowerShell 分别执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\invoke_grok_acp_fake_tool_probe.ps1 `
  -OutputDirectory .\.observed-runs\acp-fake-tool-allow-next `
  -Scenario allow_once

powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\invoke_grok_acp_fake_tool_probe.ps1 `
  -OutputDirectory .\.observed-runs\acp-fake-tool-cancel-next `
  -Scenario cancel_permission
```

输出目录必须事先不存在。若锁定 binary 的实际通知、permission option 或 cancel 行为改变，应保留原始 artifact
并缩小 claim；不得放宽 verifier 来伪造 PASS。

## 后续边界

本方向下一项是 explicit child-process timeout fixture：先用 fake tool 生成可识别的子进程树，机械证明
timeout/cancel/parent-exit 后 Job Object 收束和输出 drain，不接真实 provider。checkpoint/delta 的独立 verifier、
shadow-Git recovery 和 compaction provenance 仍是平行未完成项。真实 DeepSeek development probe 继续需要单独的
用户授权、凭据安全置入与费用确认。
