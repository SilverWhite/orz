# Grok Windows child-tree containment probe（2026-07-23）

状态：fake-only fixture、三场景 provider、管理员 launcher、结果 Schema、独立 verifier 与 synthetic/tamper
tests 已实现；`0.2.111` 管理员级 observed 三场景均通过，`windows_child_tree_timeout` promotion gate 已满足。
`0.2.106` 对照版的 task cancel 与 parent exit 通过，但 tool timeout 在返回终态后未退出并触发外层 60 秒超时。
未调用真实 Grok/DeepSeek 模型，未登录，未修改用户级 Grok 配置。

## 目标

本 probe 使用 Grok 实际公布的 Windows 工具 schema，而不是在 Grok 外部模拟一个无关超时：

1. `run_terminal_command` 启动固定 root → child → grandchild Python 进程树；
2. `tool_timeout` 使用 tool 自身的毫秒 timeout；
3. `task_cancel` 先以 `background=true, timeout=0` 启动，再把实际返回的 task ID 传给
   `kill_command_or_subagent`，最后用 `get_command_or_subagent_output` 回收输出；
4. `parent_exit` 在 background tree 和第二次 provider 请求均已观察到后关闭 Grok 外层
   kill-on-close Job Object；
5. 三场景均要求 post-trigger nonce process scan 为零，并由独立 verifier 重算。

`child_tree_fixture.py` 为三个角色分别原子写入 PID/PPID、nonce、脚本 digest 和启动时间。正常运行需保持
300 秒；门禁要求三个记录都存在、child/grandchild 父子关系正确，且 `normal_completion=false`。因此 probe
观察的是提前 teardown，而不是短命进程自然退出。

## 安全边界

- launcher 必须使用 Administrator token；普通 Codex 沙箱授权不等于 Windows UAC 提权；
- 输出目录必须事先不存在；
- workspace、profile、配置、命令和进程记录全部位于 disposable ignored run 目录；
- fake provider 只绑定 `127.0.0.1`，响应固定且 `real_model_invoked=false`；
- exact Grok 与 Python executable 均临时阻断 non-loopback IPv4 和全部 IPv6；
- Grok 使用 clean environment，只暴露 Bash、其后台任务生命周期依赖和 Grok 强制保留的 tool-discovery
  surface；deterministic provider 只发送仓库生成的固定 Bash/cancel/output 命令；
- preflight/launch/postrun 三张 restricted workspace receipt 必须保持 control aggregate 与 scan policy；
- finally 路径删除所有 `LIFGrokChild-*` 规则；残留不为零即失败。

Python 子进程本身必须被加入防火墙，因为它是 Grok tool 启动的独立 executable。loopback provider 与 Grok
使用同一个 Python binary 时仍可通信，因为规则只覆盖 non-loopback IPv4 与 IPv6。

## 证据与独立复核

每个 observed run 产生：

- `result.json`：binary/release identity、三个角色记录、触发前后进程投影、provider tool sequence、输出
  marker、workspace receipt 与 firewall receipt；
- `verification.json`：重读全部 artifact、release metadata、provider private capture 与 workspace receipt，
  重建 tool sequence、PID 关系、输出 marker，并再次执行 nonce process scan；
- `provider/requests.private.jsonl`：只包含 fake prompt/command 的被忽略私有捕获；
- `grok.stdout.log` / `grok.stderr.log`：原始 headless stream，仅保存在 ignored run 目录。

`tool_timeout` 与 `task_cancel` 要求三个角色的 stdout/stderr ready marker 均能从 Grok/provider capture 回收；
`parent_exit` 故意终止 owner，因此只要求三角色已启动和 teardown 无残留，不把未 drain 的输出伪装成完整。

结果与 verification 分别受：

- `integration/grok/grok-windows-child-tree-probe-result-v0.1.schema.json`
- `integration/grok/grok-windows-child-tree-probe-verification-v0.1.schema.json`

约束。Schema PASS 和 verifier PASS 只证明固定 fixture 的机械一致性，不证明通用进程安全，更不证明科学正确性。

## Observed run 矩阵

2026-07-23 从管理员 PowerShell 为 baseline 与 candidate 各运行三场景。候选版三项的
`result.valid=true`、`verification.valid=true`、post-trigger residue 与 firewall residue 均为零：

| 版本 | 场景 | 最终 run | 结果 | result SHA-256 | verification SHA-256 |
|---|---|---|---|---|---|
| `0.2.111` | `tool_timeout` | `child-tree-0.2.111-tool-timeout-v3` | PASS | `289c61f3447115fafe0d3f06a8d3608391693a68b3164746c17e956814eafbaf` | `71bedab122be31b7774f953f0ba2e1bce0c3adb50c1c51490c280c132298055b` |
| `0.2.111` | `task_cancel` | `child-tree-0.2.111-task-cancel-v8` | PASS | `ea230cf8985813bee09008f2c50b182fa66df9208917349a32c17c87e9cb5ecd` | `33b43f9551d4b630dbe19648bd4d10db182c307168466fc5038aa4eacae8ea10` |
| `0.2.111` | `parent_exit` | `child-tree-0.2.111-parent-exit-v1` | PASS | `3e0b8ef697cdd81f2f51ffba1ade02d6a26d30cd8251a49277502bcd8c8704b3` | `2cc08349694abfa823f3535aef1a55d8929b115657e94873c19385a1c86dba31` |
| `0.2.106` | `tool_timeout` | `child-tree-0.2.106-tool-timeout-v1` | FAIL：Grok 返回终态后仍未退出 | failure `106e125440c813362800f1b380a3660943572bade35ce6db8335e6599424d9c1` | firewall `78155bf450a77abd2eb3c8669e91830d44e0f0da8e59b14745a02e05cd22d7d7` |
| `0.2.106` | `task_cancel` | `child-tree-0.2.106-task-cancel-v1` | PASS | `2bf178963db8cf6bdcd6eb20743e9af586f673c7b5e350627a12c4aae1108ce1` | `c81d080f859afbb6fec20a467973eee8a8c873ced268cad32089b65ae04755f7` |
| `0.2.106` | `parent_exit` | `child-tree-0.2.106-parent-exit-v1` | PASS | `19461bbe38186420b5cd8303206a94a3ecba6c31bfb618683a39ca58b3c23186` | `3edeeba85be0aa19ac4308f6ac530176cf113e6ed4dc6df1c69e72880a9d7806` |

候选版每个 PASS run 都观察到 3 个角色、4 个 nonce 相关进程、触发后 0 个进程；`task_cancel`
的实际 tool sequence 为
`run_terminal_command → kill_command_or_subagent → get_command_or_subagent_output`。基线失败 run
仍由 finally 关闭外层 Job，且临时防火墙规则已删除、残留为零。

复现实测：

```powershell
$versions = @(
  @{ Name = '0.2.106'; Metadata = '.\upstream\grok-build.lock.json' },
  @{ Name = '0.2.111'; Metadata = '.\upstream\grok-build.candidate.json' }
)
$scenarios = @('tool_timeout', 'task_cancel', 'parent_exit')
foreach ($version in $versions) {
  foreach ($scenario in $scenarios) {
    powershell -NoProfile -ExecutionPolicy Bypass `
      -File .\scripts\invoke_grok_windows_child_tree_probe.ps1 `
      -OutputDirectory ".\.observed-runs\child-tree-$($version.Name)-$scenario-v1" `
      -Scenario $scenario `
      -ReleaseMetadataPath $version.Metadata
    if ($LASTEXITCODE -ne 0) { throw "failed: $($version.Name) / $scenario" }
  }
}
```

不要预建输出目录。任何失败 run 必须保留其 `failure.json` 与 raw artifacts；不得通过放宽 PID、tool sequence、
output-drain 或 residue 规则制造 PASS。

实现调试期间的失败链也保存在 `.observed-runs`。它依次暴露了 native-array PowerShell 参数绑定、非 UUID
session ID、Bash-only allowlist 禁用后台能力、Grok UUID task-id 解析、read-like 生命周期权限分类和
pre-trigger 观测竞态；修复均收紧或补全证据链，没有放宽通过条件。

## Promotion 裁决

candidate `0.2.111` 三场景的 `result.valid=true`、独立 `verification.valid=true`、防火墙残留为零已经满足；
仓库回归通过后可把 `windows_child_tree_timeout` 改为 `passed` 并把候选状态改为 `eligible`。默认版本切换仍需
单独 promotion 提交；本 probe 实现与验收提交不夹带版本提升。
