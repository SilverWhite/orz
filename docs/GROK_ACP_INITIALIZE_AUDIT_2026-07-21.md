# Grok ACP initialize 无模型探针审计（2026-07-21）

状态：锁定 Windows binary 的一次真实 `grok agent stdio` 观察已通过；不修改 LIF protocol v0.1、
reason code、gate 或 claim 语义，也没有创建 session 或模型 turn。

## 结论

Grok `0.2.106` 在 ACP `initialize` 请求中同意 protocol version `1`，返回结构化 capability 与 x.ai
扩展 metadata。探针发送的 `clientCapabilities` 为 `{}`，准确表示当前 probe 没有实现 client-side
filesystem/terminal handler。请求后没有发送 `session/new`、`session/prompt` 或其他 JSON-RPC request。

成功运行位于被 Git 忽略的 `.observed-runs/acp-initialize-v8/`。核心摘要：

- binary SHA-256：`a6a25d55daadca0c2458a5aceb4c1873eb7c76964ef307647d079e344c53969a`；
  Authenticode `Valid`，签发主体包含 `CN=X.AI LLC`；
- result SHA-256：`2fdc184b4a22e405f4c4d587958f89d2c02d940cfeef8153bde4a6df09432fdb`；
- verification SHA-256：`c61ecbda0db11d85999cc9535c03f3694f39c5a63c9d524affd447a35f6b63af`；
- request/response SHA-256：`eed65bb445e52cd1c0f5c55b57da37cfc5f300efffe7101f5571c60ac3ceb89c` /
  `2a1089b54047ede67256fd75987b45bec8d8a8e39408a1d9fe3d4f5d4287fc1c`；
- result 与独立 verifier 全部检查为真，stderr 为 0 字节，leak hit、残留 firewall rule、残留临时
  workspace 均为 `0`；进程在响应后通过 kill-on-close Job Object 有界终止。

## 观察到的 capability

`agentCapabilities` 的非空叶节点为：

- `loadSession=true`；
- `promptCapabilities.image=false`、`audio=false`、`embeddedContext=true`；
- `mcpCapabilities.http=true`、`sse=true`；
- `_meta.x.ai/fs_notify=true`；
- `_meta.x.ai/hooks.blockingEvents=["pre_tool_use"]`；
- `_meta.x.ai/hooks.decisions=["deny"]`。

原始响应还包含空的 `sessionCapabilities` 与 `auth` 对象、一个 `grok.com` auth method，以及 32 个
result `_meta` 叶路径。`_meta` 宣告当前模型、命令、MCP、cancel/recap/voice 等状态只是 initialize
metadata；它不证明模型被调用或这些能力已经通过 LIF conformance。

响应后还观察到一条无 `id` 的 `_x.ai/mcp/servers_updated` 通知，参数精确为
`{"mcpServers":[]}`。launcher 只允许这一精确空形状；独立 verifier 重读原始 JSONL，要求第一行与
response artifact 相等，并拒绝非空服务器、其他 method、额外字段、stderr 或未登记行。

## 安全与可重放边界

launcher 在空的 `C:\tmp\lif-acp-init-*` workspace 中运行，前后各生成 restricted trust receipt；
两次 candidate count 都为 `0`，aggregate 均为空内容 SHA-256。子进程环境先完全清空，再加入最小 Windows
变量、隔离 HOME/profile/temp、无 credential 的 `GROK_HOME` 和 fail-closed proxy。管理员级 Windows
Firewall 规则阻断锁定 `grok.exe` 的所有出站连接，规则创建失败则不启动，`finally` 后必须为零残留。

[`verify_grok_acp_initialize_probe.py`](../scripts/verify_grok_acp_initialize_probe.py) 不信任 result 的投影：
它重新校验 schema、binary/request/response/transcript/receipt 哈希，重建 capability/meta 叶路径，并再次证明
initialize-only 请求、JSON-RPC id/version、空稳定 workspace 与全部安全检查。

## 调试链与所得 Windows 约束

正式 `v8` 之前的运行保留在忽略目录，未被提升为最终成功证据。它们暴露了 Windows PowerShell 5.1 的三个真实
工程边界：脚本参数默认值中的 `$PSScriptRoot` 求值时机、空 generic list 的 mandatory binding、以及空文件
`Get-Content -Raw` 返回 `$null`。另一个失败来自把 restricted trust receipt 错配了仅 trusted 模式接受的
expected digest；`v6` 则由 verifier 正确拒绝了尚未分类的扩展通知。这些修复都没有改变 ACP request 或
provider 协议语义。

## 未证明项与下一步

本轮只证明 initialize。它没有证明 `session/new`、模型 prompt、tool/permission 生命周期、cancel、后台子进程、
compaction、session file 对账，也没有证明 Global Progress Sentinel 已注入 Grok runtime。下一 spike 仍使用
loopback fake provider 与 disposable workspace，验证 ACP `tool_call` → `tool_call_update` → 唯一 terminal，
并把 permission/cancel 与 session `events.jsonl`/`updates.jsonl` 作为交叉证据；真实 DeepSeek 调用继续需要单独授权。

协议依据：[ACP initialization specification](https://agentclientprotocol.com/protocol/v1/initialization)；锁定上游
agent mode 文档见
[`MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`](../architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md)。
