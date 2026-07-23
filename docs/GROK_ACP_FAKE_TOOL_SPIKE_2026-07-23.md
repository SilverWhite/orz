# Grok ACP fake-tool continuity spike（2026-07-23）

状态：实现与 synthetic verifier fixture 已完成；管理员级 Windows live smoke 待执行。未调用真实
Grok/DeepSeek 模型，未写真实用户 workspace，未提升任何 FEP/LIF claim。

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
`15-agent-mode.md` 仍是 binary-specific 请求顺序依据；网站当前 v1 的后续新增字段不自动扩大本锁定版本 claim。

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
- provider 只收到第一次 tool-call request；
- session event 不得出现 `tool_completed`。

## 当前机械证据

- 新增 Python 文件通过 `py_compile`；
- launcher 通过 PowerShell AST parse；
- synthetic verifier fixture 覆盖 allow、cancel 和 transcript 篡改，3 tests passed；
- fake provider 覆盖 single、continuity、cancel 与 overwrite refusal，4 tests passed；
- repository checker 当前识别 41 schemas、40 cases，error count 0；
- `fep-script-validation` 以 `legacy_review --task-board --exit-zero` 检查三个 Python 入口：
  全部 0 FAIL、0 catastrophic red line。seed、checkpoint、numeric dynamics 等 WARN 属于 experiment-domain
  mismatch，不通过添加伪字段消除。

以上只证明实现的静态/合成边界。当前 Codex 进程
`SWITCH\CodexSandboxOffline` 没有 Administrator token，launcher 已实测在创建输出目录前 fail closed；
因此本文件不登记 live Grok PASS。

## 管理员验收命令

从管理员 PowerShell 分别执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\invoke_grok_acp_fake_tool_probe.ps1 `
  -OutputDirectory .\.observed-runs\acp-fake-tool-allow-v1 `
  -Scenario allow_once

powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\invoke_grok_acp_fake_tool_probe.ps1 `
  -OutputDirectory .\.observed-runs\acp-fake-tool-cancel-v1 `
  -Scenario cancel_permission
```

只有两份 `result.json` 和 `verification.json` 都为 `valid=true`、临时规则残留为零，才能把本 spike 改标为
Windows observed。若锁定 binary 的实际通知、permission option 或 cancel 顺序不同，应保留原始 artifact 并缩小
claim；不得放宽 verifier 来伪造 PASS。

## 后续边界

管理员 smoke 通过后，本方向下一项才是 explicit child-process timeout fixture。checkpoint/delta 的独立 verifier、
shadow-Git recovery 和 compaction provenance 仍是平行未完成项。真实 DeepSeek development probe 继续需要单独的
用户授权、凭据安全置入与费用确认。
