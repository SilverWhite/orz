# Grok Build integration fixtures

状态：被动配置与 conformance 入口；不是已启用的 provider 配置。

真实 DeepSeek 直连已经通过独立 one-shot transport conformance。下一层
[`invoke_grok_real_deepseek_conformance.ps1`](../../scripts/invoke_grok_real_deepseek_conformance.ps1)
现提供 Grok Build 的两阶段薄 launcher：Plan 完全离线；Execute 固定单 turn、无工具、
read-only、无 debug，并在 launcher 内部从 Windows Credential Manager 向 Grok 子进程
注入短时环境变量。当前只完成离线/fail-closed 验证，尚未执行真实 Grok provider request。
详细边界见
[`GROK_REAL_DEEPSEEK_LAUNCHER_2026-07-23.md`](../../docs/GROK_REAL_DEEPSEEK_LAUNCHER_2026-07-23.md)。

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

## ACP fake-tool continuity probe

[`invoke_grok_acp_fake_tool_probe.ps1`](../../scripts/invoke_grok_acp_fake_tool_probe.ps1) 在 initialize 探针之上新增
两个固定 fake-only prompt-turn 场景：

- `allow_once`：响应 `session/request_permission` 的 allow-once option，要求同一 `toolCallId` 从
  `tool_call` 到唯一 completed terminal，并与 provider 第二轮和 session files 对账；
- `cancel_permission`：permission pending 时发送 `session/cancel` 和 `cancelled` outcome，要求 prompt
  返回 `cancelled`、provider 没有第二次请求、session 没有 `tool_completed`。

launcher 继续要求 Administrator token、三张 restricted workspace receipt、已锁定 binary 的 non-loopback
IPv4/全 IPv6 临时阻断、clean environment、read-only sandbox 和 kill-on-close Job Object。原始 ACP transcript
只保存在被忽略的 fake fixture 目录；result 只投影 method、ID、status、计数与 artifact digest。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ..\..\scripts\invoke_grok_acp_fake_tool_probe.ps1 `
  -OutputDirectory ..\..\.observed-runs\acp-fake-tool-allow-v1 `
  -Scenario allow_once
```

当前代码、Schema、synthetic verifier/tamper tests 与管理员级 Windows fake-only live smoke 均已通过；
allow/cancel 结果经独立 verifier 重建，临时防火墙规则残留为零，且没有真实模型调用。验收边界、取消竞态和
artifact digest 见
[`GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md`](../../docs/GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md)。

## Windows child-tree containment probe

[`invoke_grok_windows_child_tree_probe.ps1`](../../scripts/invoke_grok_windows_child_tree_probe.ps1) 使用 Grok
实际公布的 `run_terminal_command`、`kill_command_or_subagent` 与
`get_command_or_subagent_output` schema，驱动固定 root → child → grandchild 进程树。三个场景分别覆盖
tool timeout、background task cancel 和关闭 Grok 外层 Job Object 的 parent exit。

launcher 要求显式传入 baseline 或 candidate release metadata；它不会自动选择 latest，也不会修改默认版本。
exact Grok/Python executable 均临时阻断 non-loopback IPv4 与全部 IPv6，provider 只绑定 loopback，三张
workspace receipt 和 finally firewall cleanup 均为 hard gate。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File ..\..\scripts\invoke_grok_windows_child_tree_probe.ps1 `
  -OutputDirectory ..\..\.observed-runs\child-tree-0.2.111-tool-timeout-v1 `
  -Scenario tool_timeout `
  -ReleaseMetadataPath ..\..\upstream\grok-build.candidate.json
```

当前代码、结果/verification Schema、synthetic/tamper tests 与管理员 observed 矩阵均已完成；
candidate `0.2.111` 三场景全部通过，promotion gate 为 passed，并已被选择为默认版本。baseline `0.2.106`
仅 tool-timeout 路径在返回终态后未退出。完整边界、digest 与六次 baseline/candidate 命令见
[`GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`](../../docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md)。

## Compaction provenance probe

[`invoke_grok_compaction_provenance_probe.ps1`](../../scripts/invoke_grok_compaction_provenance_probe.ps1)
在隔离 profile 中驱动三次固定主请求：首轮写入来源 canary，第二轮执行 `/compact`，第三轮验证压缩后继续。
全局 hook 记录 `PreCompact`/`PostCompact`；探针另对 `chat_history` source span、compaction request、
checkpoint、session files 和 provider capture 计算 digest。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File ..\..\scripts\invoke_grok_compaction_provenance_probe.ps1 `
  -OutputDirectory ..\..\.observed-runs\grok-compaction-provenance-v1 `
  -ReleaseMetadataPath ..\..\upstream\grok-build.lock.json
```

Grok `0.2.111` 的管理员 fake-only observed run 已通过独立 verifier 16/16 checks；摘要状态固定为
`derived_unverified`，不能替代原始 observation。runtime 未直接给出的 retained/discarded 逐 item 映射保持
`unknown`。该探针不调用真实模型，且只覆盖 manual `/compact`，详见
[`GROK_COMPACTION_PROVENANCE_2026-07-23.md`](../../docs/GROK_COMPACTION_PROVENANCE_2026-07-23.md)。

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
