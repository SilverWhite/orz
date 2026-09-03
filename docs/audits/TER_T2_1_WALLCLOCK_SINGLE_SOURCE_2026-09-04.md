# TER T2.1 墙钟单一化实施审计（2026-09-04）

> 范围：TODO2 M2 步 T2.1——删 Windows runner `--max-wallclock` 与
> `timeout-60` 余量（840 隐形墙钟）；sandbox `--timeout` = 官方
> `agent_timeout_seconds` 为唯一评测墙钟；F6 pull/push 读源以 runner
> 施加值为准。验收 = DryRun 确认无 840 参数、逐题 timeout 生效。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §2 P2/§3.3/§10 S2-1；背景证据（chunk1-0303 840s 墙钟）见设计稿 §1 与
> `_windows_high_nist/analysis/`。

## 1. 改动

`_windows_high_nist/run/run_agent_arm.ps1`（VM agent runner）：

- 删除 `$wallclockSec = [Math]::Max(60, $perTaskTimeoutSeconds - 60)` 派生
  与 `sbxArgs` 中 `--max-wallclock "$wallclockSec"`（840 类隐形余量墙钟
  移除）。
- sandbox `--timeout "$perTaskTimeoutSeconds"`（= task.json 官方
  `agent_timeout_seconds`，缺省 960）保持为唯一评测墙钟。
- 官方超时经 `$env:ORZ_MAX_WALLCLOCK = "$perTaskTimeoutSeconds"` 透传给
  orz 内部——F6 pull/push（T1.8/T1.9）读源以 runner 施加值为准；任务
  循环后清理该 env。
- DryRun 输出新增
  `AGENT_DRYRUN wallclock runner_imposed=sandbox --timeout <N>
  max_wallclock_arg=none env_ORZ_MAX_WALLCLOCK=<N>`（计划可审）。

## 2. 验收证据（DryRun，真实两题 900s/3600s）

计划输出（摘要）：

```text
AGENT_DRYRUN TASK=t900 env=process-env allowlist= appcontainer=off
AGENT_DRYRUN SBX: … --arm control --timeout 900 --output … --command … 
  -p do thing --real --allow-write --allow-shell --allow-network --run-root …
AGENT_DRYRUN wallclock runner_imposed=sandbox --timeout 900 max_wallclock_arg=none env_ORZ_MAX_WALLCLOCK=900
AGENT_DRYRUN TASK=t3600 …
AGENT_DRYRUN wallclock runner_imposed=sandbox --timeout 3600 max_wallclock_arg=none env_ORZ_MAX_WALLCLOCK=3600
AGENT_ERRORS=0
AGENT_DONE
```

- 无 `--max-wallclock`、无 840（输出扫描零命中）；
- 逐题 timeout 生效：900 / 3600 分别落在 sandbox `--timeout` 与
  `ORZ_MAX_WALLCLOCK` env；
- 完整 DryRun 日志与临时任务集位于本地 %TEMP%（已清理）。

## 3. 边界声明

- orz-bin `--max-wallclock` CLI 保留为本地显式逃生阀（ORZ_MAX_WALLCLOCK
  env 由 runner 透传官方值），评测路径不再由 runner 派生 840 余量；
- 旧 `scripts/rerun_failures_k1.ps1` 与 `__wtest.ps1` 中的 840 属历史
  Linux/诊断 harness（不在 Windows runner 施加链），登记不改；
- M2 T2.3/T2.4 实机验证沿用本 runner（DryRun + 实机）承接。

## 4. 仓库纪律说明

本文件在工作区夹带 0l 任务未提交的 `ORZ_AGENT_KEY_FILE` 改动；按任务
隔离纪律，TER 提交前已临时摘出该 0l 块，提交后原样放回工作区（不混入
TER 提交）。
