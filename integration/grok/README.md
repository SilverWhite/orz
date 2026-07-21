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
