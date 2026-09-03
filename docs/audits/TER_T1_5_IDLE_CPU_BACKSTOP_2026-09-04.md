# TER T1.5 idle+CPU 兜底实施审计（2026-09-04）

> 范围：TODO2 M1 步 T1.5——后台任务进程监视采样器（输出字节增长 + CPU
> 时间）；连续 300s 无活跃 → kill + 提醒文本 + `idle_killed` 形态；阈值
> 参数化；验收 = 模拟「无输出计算」与「真 idle」两类用例、不误杀前者。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.1（活跃判定兜底口径）/ §7（5min + CPU 活跃辅助定案）/ §10 S1-3；
> 事件契约见 T0.2（schema/verifier/fixtures 的
> `tool_running status=idle_killed + reason`）；前置步骤 T1.4 审计同目录。

## 1. 目标与验收

- 后台任务连续 `idle_kill_timeout`（默认 300s = 5min）「无输出字节增长且
  进程树 CPU 时间不增」→ 机械 kill，exit 形态 `idle_killed`，完成提醒带
  自描述杀因文本。
- 输出增长为主、CPU 活跃辅助：编译/训练/渲染等「无输出计算」不误杀。
- 阈值参数化：backend 级配置（actor 字段 + `GROK_IDLE_KILL_TIMEOUT_MS`
  env + 测试构造器；`0` = 禁用）。
- CPU 记账不可用平台（非 Windows/Linux）按「CPU 未知 = 不判 idle」处理，
  宁可不杀也不误杀。
- 验收点：采样器数学用例（无输出计算/真 idle/慢输出/禁用/CPU 未知）+
  实机进程用例（sleep 被 idle-kill、忙循环不被杀）+ 提醒文本用例。

## 2. 代码改动

### 2.1 xai-tty-utils（进程树 CPU 读数）

`crates/codegen/xai-tty-utils/src/lib.rs`：

- `ProcessGroup::cpu_time()`：返回 `io::Result<Option<Duration>>`——
  Windows 用 Job Object 记账（`QueryInformationJobObject` Basic
  Accounting，覆盖 Job 内全部后代进程）；Linux 扫 `/proc/*/stat` 汇总
  同 pgrp 的 utime+stime（进程组树级）；其它平台 `Ok(None)`。
- `ProcessGroup::pgid()`（unix）：暴露 killpg-safe 组 id 供 /proc 扫描。
- Windows 100ns 计数换算 helper。
- 依赖沿用 workspace `windows`（已含 JobObjects/Foundation/Threading
  features），无 Cargo 变更。

### 2.2 orz-tools 终端 actor（`computer/local/terminal.rs`）

- 常量与配置：`DEFAULT_IDLE_KILL_TIMEOUT=300s`（pub(crate)，提醒层共用）、
  `IDLE_CPU_SAMPLE_INTERVAL=1s`、`IDLE_KILL_SIGNAL="idle_killed"`、
  `idle_kill_reason(timeout)`（与 T0.2 夹具口径一致）；actor 级
  `idle_kill_timeout` 字段贯穿 `LocalTerminalConfig` /
  `new_with_ttl` / `new_inner` / 测试构造器（新增
  `new_with_idle_kill_timeout`）。
- `ActivitySampler`（每进程）：`tick(now, total_bytes, cpu_micros)`——
  输出字节每 tick 比较；CPU 累计按 1s 节流读取；首个样本只建基线；
  连续无活跃满阈值返回 kill；输出/CPU 增长复位 idle 计时；
  `Duration::ZERO` 禁用；CPU 记账未知不判 idle。
- `ProcessState` 增 `activity` 字段（前台/后台两处 spawn 均初始化）。
- `poll_process`：后台化且运行中的任务每 tick 采样（先 `try_wait` 排除
  本 tick 已自然退出的任务，避免自然退出被误记 idle-kill）；命中 →
  `idle_kill_background_task()`：SIGTERM 起手（poll 循环既有路径升级
  SIGKILL）、exit status 记 `idle_killed`、flush 落盘、notify waiters；
  随后既有 poll 完成通知路径发 `TaskCompleted`（snapshot
  `signal=idle_killed`）。

### 2.3 形态与提醒文本

- `grok_build/bash/mod.rs`：`KillReason` 增 `IdleKilled`
  （parse/display `idle_killed`）——前台/完成 prompt 头渲染
  `exit: killed (idle_killed)`。
- `reminders/task_completion.rs`：`format_bash_completion` 对
  `signal=idle_killed` 渲染 `idle-killed (no output growth or CPU
  activity for 300s)`（T0.2 夹具同口径），不再显示泛化
  `terminated by signal idle_killed`。

## 3. 测试

| 位置 | 用例 | 覆盖 |
|---|---|---|
| terminal.rs | `activity_sampler_kills_only_after_continuous_quiet` | 连续安静满阈值才 kill |
| terminal.rs | `activity_sampler_output_growth_resets_idle` | 慢输出复位 idle 计时 |
| terminal.rs | `activity_sampler_cpu_growth_prevents_idle_kill` | 无输出计算不误杀（数学） |
| terminal.rs | `activity_sampler_unknown_cpu_never_kills` | CPU 未知平台安全侧 |
| terminal.rs | `activity_sampler_zero_timeout_disables` | 0 = 禁用 |
| terminal.rs | `test_idle_background_task_killed_after_quiet_period` | 实机：`sleep` 后台任务被 idle-kill，snapshot `signal=idle_killed` |
| terminal.rs | `test_busy_no_output_task_not_idle_killed` | 实机：无输出忙循环越过阈值仍存活（Windows PowerShell / Linux bash 双语法），随后 kill 清理 |
| bash/mod.rs | `default_prompt_killed_reasons` | 增 `idle_killed` 头渲染 |
| task_completion.rs | `format_bash_completion_idle_killed_lists_reason` | 完成提醒自描述杀因 |

## 4. 验证证据

- `cargo test -p orz-tools --lib computer::local::terminal::tests`：
  **47 passed / 0 failed**（5 ignored 为既有 flaky 标注；含 5 个采样器
  用例 + 2 个实机进程用例）。
- `cargo test -p orz-tools --lib bash`：**231 passed / 0 failed**。
- `cargo test -p orz-tools --lib reminders::task_completion`：
  **49 passed / 0 failed**；`kill_reason_parse_and_display_round_trip`
  1 passed。
- `cargo fmt -p orz-tools -p xai-tty-utils -- --check`：净（exit 0）。
- orz 仓库 `git diff --check`：exit 0（CRLF→LF 提示为既有/换行归一警告，
  非错误）。
- 说明：Windows 沙箱全量 `--lib` 的 grep/glob 外部命令失败属环境性
  （T1.3 已登记）；Linux 全量 + clippy 归 M1 门 T1.13。

## 5. 边界声明

- 本步落地终端侧 idle-kill 机制与提醒/形态；`tool_running`
  `status=idle_killed` 的 **journal 侧接线**（快照 idle-killed → 同 run
  记一条 `tool_running(idle_killed, reason)`，须后于 running:true
  completion）随事件链集成处理，归 T1.13 门（T0.2 schema/verifier/
  fixtures 已就绪，生产者在本步 terminal 快照层具备全部事实：
  call_id=task_id、output_file、total_bytes、signal=idle_killed）。
- 完成提醒文本按 resident 默认 300s 渲染（snapshot 无 idle 时长字段）；
  非默认阈值下的精确 idle 时长由 tracing/审计字段承载。T1.6 processes
  live 分区如需要精确 idle 时长，届时扩展快照字段一并处理。
- 10h 绝对兜底与显式 kill 语义不变；idle-kill 只作用于后台化任务
  （前台非自动后台路径由预算/逃生阀 timeout 管辖，T1.4 已定）。
