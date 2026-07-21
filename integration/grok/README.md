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

## Loopback fake-provider conformance

[`fake_deepseek_provider.py`](../../scripts/fake_deepseek_provider.py) 只绑定 `127.0.0.1`，接受一次
Chat Completions 请求并返回固定 SSE。它保存完整 request body 到被忽略的 fixture-private artifact，但所有
header value（尤其 Authorization）只保留 presence/长度/digest，不保存原值。因此它只能使用仓库内惰性
prompt，不能承载敏感输入。

[`invoke_grok_fake_provider_conformance.ps1`](../../scripts/invoke_grok_fake_provider_conformance.ps1) 要求管理员
令牌，在启动 Grok 前为已核验 binary 创建临时出站阻断规则：排除 `127.0.0.0/8` 后阻断全部 IPv4，并阻断
全部 IPv6。它还清空子进程继承环境，只重新加入最小 Windows 环境、隔离的 HOME/profile/temp、假 credential
和 fail-closed proxy。规则在 `finally` 中按 run UUID 删除；创建规则失败时 Grok 不启动。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ..\..\scripts\invoke_grok_fake_provider_conformance.ps1 `
  -OutputDirectory ..\..\.observed-runs\fake-provider-smoke
```

锁定的 Grok `0.2.106` 曾在被拒绝的 pre-fix probe 中把假 Authorization value 写入 `--debug-file`。
launcher 因此禁用该参数，并扫描全部 run artifact，credential value 命中即 FAIL。当前 fixture 单元测试已
通过；修复后的管理员 smoke 已通过。完整失败链、最终 run ID 与 artifact digest 见
[`GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md`](../../docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md)。
