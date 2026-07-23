# Grok real DeepSeek thin launcher（2026-07-23）

状态：two-stage implementation、PowerShell AST、offline plan、sandbox-user
fail-closed 与 Schema/static tests 已完成；尚未授权或执行真实 Grok→DeepSeek 请求。

## 1. 正式职责边界

该入口不实现第二套 model/session/tool runtime。Grok Build `0.2.111` 继续拥有 model
transport、headless session、permission 和 sandbox；本仓库只负责：

- 根据 checked-in lock 重新核验 binary 长度、hash、Authenticode 和 version；
- 离线冻结一次请求的确认摘要；
- 创建隔离 profile、temp 和零 control-surface workspace；
- 在 launcher 内部用 `CredReadW` 读取固定 Generic Credential
  `FEP-Agent/DeepSeek`；
- 只向 Grok 子进程注入 `LIF_DEEPSEEK_API_KEY`，不把 key 放入命令行、配置、PowerShell
  参数或普通 artifact；
- 禁用 debug、memory、subagents、web 和全部 tools；固定 `max-turns=1`、
  `permission-mode=plan`、`sandbox=read-only`；
- 使用 kill-on-close Job Object、有界等待、双 workspace receipt 和 terminal
  artifact leak scan。
- 原始 Grok stdout/stderr 只在 launcher 内存中解析；artifact 仅保存长度、SHA-256、
  marker/usage 投影，不保存原始流。

## 2. 两阶段入口

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\scripts\invoke_grok_real_deepseek_conformance.ps1 `
  -Mode Plan `
  -OutputDirectory .\.observed-runs\grok-real-deepseek-v1
```

Plan 阶段不读 credential、不启动 Grok、不联网。只有重新提供输出中的
`ALLOW-GROK-<digest 前12位>`，Execute 才会继续：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\scripts\invoke_grok_real_deepseek_conformance.ps1 `
  -Mode Execute `
  -OutputDirectory .\.observed-runs\grok-real-deepseek-v1 `
  -ConfirmationToken ALLOW-GROK-XXXXXXXXXXXX
```

## 3. 当前离线证据

- PowerShell AST parse PASS；
- offline plan 固定 `grok 0.2.111 (94172f2aa4)` 和 binary SHA-256；
- 完整回归：50 prototype + 42 Grok integration + 6 runtime=`98` tests PASS；
- repository checker：52 schemas、0 errors；
- 沙箱账户 execute 到达 `credential_acquire_and_process_start` 后 fail closed；
- 该失败中 `grok_started=false`、provider/billing=`not_attempted`、retry=`0`；
- 静态测试拒绝 raw key 参数、环境继承和 debug-file，并检查固定 endpoint/model、
  no-tool/read-only/Job Object/WER/leak-scan 控制；
- 专用 plan/result/failure Schema 已加入仓库检查；terminal scan 包含待原子写入的
  JSON document。

## 4. 保留限制

- API key 必须存在于 Grok 子进程环境；launcher 的 WER `NOHEAP` 不会自动禁止 Grok
  子进程自身的 crash dump；
- .NET/Win32 创建环境块仍会产生不可密码学零化的 immutable copy；
- Job Object 在进程启动后立即分配，保留已知的短 pre-assignment race；
- pattern scan、clean environment 和禁用 debug 显著缩小落盘泄漏面，但不能覆盖
  pagefile、hibernation、管理员、debugger、malware、内核或 provider 系统；
- 当前只允许固定 marker、单 turn、无工具的 development conformance；真实成功前不得
  扩张为任意 prompt、真实 workspace 或通用 Agent 使用。
