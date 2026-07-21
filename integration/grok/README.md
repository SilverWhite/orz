# Grok Build integration fixtures

状态：被动配置与 conformance 入口；不是已启用的 provider 配置。

[`deepseek-custom-model.example.toml`](deepseek-custom-model.example.toml) 只证明 Grok Build 的
custom-model 配置能够表达 DeepSeek 的模型名、官方 base URL、Chat Completions backend 和
环境变量凭据入口。它故意不设置 `[models].default`，也不包含 `api_key`。

在把它复制到用户级 `.grok/config.toml` 前，必须针对
[`upstream/grok-build.lock.json`](../../upstream/grok-build.lock.json) 锁定的版本验证：

1. 最终请求路径为 DeepSeek 官方 `/chat/completions`；
2. 请求包含显式 thinking 配置和允许的 `reasoning_effort`；
3. tool call 后续轮次保留 provider 要求的 `reasoning_content`；
4. 普通 Grok session/log 不泄漏原始 private reasoning；
5. 凭据仅注入被监督的子进程环境，不写入仓库、配置模板或普通日志。

只要其中一项尚未通过，该文件就保持 discovery-only；不得通过真实付费调用来“边跑边猜”。

项目内 binary 可用以下无网络检查重复核验：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ..\..\scripts\inspect_grok_install.ps1
```

该脚本只检查 lock、文件长度/hash、PE header、Authenticode 和 `grok --version`，不登录或调用模型。

## ACP initialize no-model probe

[`invoke_grok_acp_initialize_probe.ps1`](../../scripts/invoke_grok_acp_initialize_probe.ps1) 在一次性空 workspace、
clean environment、kill-on-close Job Object 和针对锁定 `grok.exe` 的临时全出站阻断规则中，只发送一个 ACP
`initialize`。`clientCapabilities={}`，不创建 session，不发送 prompt，也不读取 credential。脚本需要管理员
令牌；通过普通 Codex 沙箱授权不等于 Windows UAC 提权。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ..\..\scripts\invoke_grok_acp_initialize_probe.ps1 `
  -OutputDirectory ..\..\.observed-runs\acp-initialize-local `
  -TemporaryWorkspaceParent C:\tmp
```

输出受 [`grok-acp-initialize-probe-result-v0.1.schema.json`](grok-acp-initialize-probe-result-v0.1.schema.json)
约束，并由 [`verify_grok_acp_initialize_probe.py`](../../scripts/verify_grok_acp_initialize_probe.py) 独立重算
artifact hash、JSON-RPC 语义、capability/meta 投影和 workspace receipt。锁定 build 在响应后会发送精确的
`_x.ai/mcp/servers_updated` 空列表通知；任何非空服务器、额外 method/field、stderr 或其他 stdout 行均失败。
正式 Windows 结果、调试失败链与未证明项见
[`GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md`](../../docs/GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md)。

## Observed dry-run

[`new_grok_observed_dry_run.ps1`](../../scripts/new_grok_observed_dry_run.ps1) 生成一个
[`grok-observed-plan-v0.1.schema.json`](grok-observed-plan-v0.1.schema.json) 约束的只读计划。它只核验本地
binary、计算输入/Git 状态 digest、冻结控制参数并原子写入新的输出目录；不会启动 Grok session、读取密钥值、
发起网络请求或执行工具。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ..\..\scripts\new_grok_observed_dry_run.ps1 `
  -PromptFile .\examples\observed-dry-run-prompt.txt `
  -ConfigPath .\deepseek-custom-model.example.toml `
  -OutputDirectory ..\..\.observed-runs\first-plan `
  -ModelAlias lif-deepseek-v4-pro `
  -ReasoningEffort high `
  -MaxTurns 2 `
  -SandboxProfile read-only `
  -NetworkPolicy disabled
```

输出目录必须事先不存在，防止覆盖旧 provenance。计划中的 `network_policy` 和空 tool allowlist 在此阶段只
是待验证意图，不能被描述成 Grok 已执行的隔离保证。

## Workspace trust preflight

[`new_grok_workspace_trust_receipt.ps1`](../../scripts/new_grok_workspace_trust_receipt.ps1)
在启动 Grok 之前静态扫描 repo root 到当前工作目录之间的 project instructions、rules、permission
config、hook、plugin、skill、agent、MCP/LSP 等候选控制面，只记录路径、类别、长度与 SHA-256，不记录
文件正文，也不执行 Grok、模型、project code 或网络请求。

```powershell
$output = '.\.observed-runs\trust-preflight.json'
powershell -NoProfile -ExecutionPolicy Bypass -File ..\..\scripts\new_grok_workspace_trust_receipt.ps1 `
  -WorkspacePath ..\.. `
  -OutputPath $output
```

默认 `restricted`：没有候选控制文件时才允许后续 launcher；存在任何候选时只生成 receipt，
`launch_permitted=false`。`trusted` 需要再次传入当前 `aggregate_sha256` 和非默认 actor；控制文件发生变化后
旧 digest 立即失效。receipt 不会写 Grok 的 `trusted_folders.toml`，也不替代 Grok 对 project
hook/MCP/LSP/plugin code 的 folder-trust。

[`invoke_grok_workspace_discovery_probe.ps1`](../../scripts/invoke_grok_workspace_discovery_probe.ps1)
会把 checked-in inert control-surface fixture 复制到一次性 Git workspace，先生成 restricted receipt，
再在隔离 profile 中运行两次本地 `grok inspect --json`。第二次附带隐藏 `--trust` 只用于观察 inspect
子命令是否签发 trust；在锁定的 `0.2.106` 上它不写 trust store，输出与普通 inspect 相同。该 probe
不启动模型或 tool session，也不能代替未来真实 session trust 路径的无 provider 验证。

## Loopback fake-provider conformance

[`fake_deepseek_provider.py`](../../scripts/fake_deepseek_provider.py) 只绑定 `127.0.0.1`，接受一次
Chat Completions 请求并返回固定 SSE。它保存完整 request body 到被忽略的 fixture-private artifact，但所有
header value（尤其 Authorization）只保留 presence/长度/digest，不保存原值。因此它只能使用仓库内惰性
prompt，不能承载敏感输入。

[`invoke_grok_fake_provider_conformance.ps1`](../../scripts/invoke_grok_fake_provider_conformance.ps1) 要求管理员
令牌，在启动 Grok 前为已核验 binary 创建临时出站阻断规则：排除 `127.0.0.0/8` 后阻断全部 IPv4，并阻断
全部 IPv6。它还清空子进程继承环境，只重新加入最小 Windows 环境、隔离的 HOME/profile/temp、假 credential
和 fail-closed proxy。规则在 `finally` 中按 run UUID 删除；创建规则失败时 Grok 不启动。

launcher 还会生成两张 restricted workspace-trust receipt：第一张早于任何 Grok 进程和 provider，第二张紧邻
Grok agent 进程创建。两张都必须完整、零候选、允许启动，而且 aggregate/scan-policy digest 一致。single 结果
使用新增 `grok-fake-provider-result-v0.2.schema.json`，tool-continuity 使用新增
`grok-tool-continuity-result-v0.3.schema.json`；旧 `0.1/0.2` schema 保留为历史格式。双检查只缩小 TOCTOU，不是
upstream folder-trust grant。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ..\..\scripts\invoke_grok_fake_provider_conformance.ps1 `
  -OutputDirectory ..\..\.observed-runs\fake-provider-smoke
```

锁定的 Grok `0.2.106` 曾在被拒绝的 pre-fix probe 中把假 Authorization value 写入 `--debug-file`。
launcher 因此禁用该参数，并扫描全部 run artifact，credential value 命中即 FAIL。当前 fixture 单元测试已
通过；修复后的管理员 smoke 已通过。完整失败链、最终 run ID 与 artifact digest 见
[`GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md`](../../docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md)。

两轮只读工具与 reasoning continuity 使用同一 launcher：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ..\..\scripts\invoke_grok_fake_provider_conformance.ps1 `
  -OutputDirectory ..\..\.observed-runs\tool-continuity-smoke `
  -Scenario tool-continuity
```

该模式只允许实际 request schema 中的 `read_file` 读取固定 fixture，并验证第二轮 assistant
`reasoning_content`、tool call ID 和 tool message。Grok 进程被分配到 kill-on-close Job Object。当前
`streaming-json` 不包含显式 tool event；完整边界与失败链见
[`GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md`](../../docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md)。

## Post-run event completeness bridge

[`invoke_grok_postrun_evidence_bridge.ps1`](../../scripts/invoke_grok_postrun_evidence_bridge.ps1) 对一个已经完成、
trust-gated 且 fake-only 的 run 做第三次 workspace receipt 复核，然后在管理员级 all-network firewall block、
clean environment 与 Job Object 下有界执行 `trace --local` 和 `export`。命令失败会写 status，不会破坏或重写
source run。

[`build_grok_event_bridge.py`](../../scripts/build_grok_event_bridge.py) 合并 stdout、session events/updates、脱敏
provider capture、supervisor result 与三张 receipt。它只输出白名单 metadata 和 raw-record digest；不复制 raw
prompt/reasoning/tool content。跨来源 runtime 顺序固定为 `not_established`，bridge sequence 只是确定性追加顺序。
[`verify_grok_event_bridge.py`](../../scripts/verify_grok_event_bridge.py) 独立重算 source/journal digest、event schema、
counts、sequence、previous/event hash 与 raw-field omission；post-run result 必须登记该 verifier report。
当前锁定 build 的 fake headless session 已落盘但未进入派生 session search index；这不影响按目录直接发现。
修复本项目 Windows PowerShell 子进程环境注入后，trace/export 均为 `available`。bridge 仍为 `partial`，因为
跨来源 runtime 顺序、sealed encryption 与显式 stdout tool lifecycle 尚未建立。完整实测、schema 和 digest 见
[`GROK_EVENT_BRIDGE_AUDIT_2026-07-21.md`](../../docs/GROK_EVENT_BRIDGE_AUDIT_2026-07-21.md)。

## Disposable fixture workspace checkpoint/delta

[`capture_grok_fixture_workspace.py`](../../scripts/capture_grok_fixture_workspace.py) 是下一阶段的 hash-only
implementation spike。它只接受含 `.lif-disposable-workspace.json` 且显式 `disposable=true` 的一次性 fixture；
输出必须位于 workspace 外。checkpoint 与 delta 分别受
[`grok-fixture-workspace-checkpoint-v0.1.schema.json`](grok-fixture-workspace-checkpoint-v0.1.schema.json) 和
[`grok-fixture-workspace-delta-v0.1.schema.json`](grok-fixture-workspace-delta-v0.1.schema.json) 约束。

它执行两次连续全量扫描，拒绝 reparse point、大小/数量超限、case-insensitive 路径碰撞、变化中的文件、marker
变化、checkpoint aggregate 不一致与输出覆盖。receipt 仅包含相对路径、长度和 SHA-256；不复制正文，也不执行
restore。created/modified/deleted 只表示两次文件系统状态之差，不声称某个 tool 是变化原因。

```powershell
python ..\..\scripts\capture_grok_fixture_workspace.py checkpoint `
  --workspace .\disposable-fixture `
  --output ..\..\.observed-runs\fixture-checkpoint.json

python ..\..\scripts\capture_grok_fixture_workspace.py delta `
  --workspace .\disposable-fixture `
  --checkpoint ..\..\.observed-runs\fixture-checkpoint.json `
  --output ..\..\.observed-runs\fixture-delta.json
```

该工具尚未接入 Grok launcher，更不允许对真实用户 workspace 自动恢复。设计边界与 Windows fixture 结果见
[`GROK_FIXTURE_CHECKPOINT_DELTA_SPIKE_2026-07-21.md`](../../docs/GROK_FIXTURE_CHECKPOINT_DELTA_SPIKE_2026-07-21.md)。
