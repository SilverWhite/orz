# TER T1.6 黑板 processes live 分区实施审计（2026-09-04）

> 范围：TODO2 M1 步 T1.6——黑板 `section=processes` live 分区：读取时
> 现算快照（task_id / 命令摘要 / elapsed / 状态 / 活跃度 / 字节 /
> 可 kill）；状态跃迁落事件（既有 tool_running / TaskCompleted 链，不
> 逐秒写事件）；kill 动作模型面可达且不暴露 `&` / `is_background`。
> 验收 = live 渲染单测 + 越权边界用例。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.2（进程注册表）/ §10 S1-4；渲染契约与越权边界见 T0.2 审计 §5；
> 前置步骤 T1.4/T1.5 审计同目录。

## 1. 目标与验收

- `blackboard_read section=processes` 提供 live-only 分区：读取时从终端
  现算（≤1s 新鲜度），行含 task_id / 命令摘要（≤80B）/ elapsed / status
  / 输出字节 / CPU / killable；整分区 ≤8KiB 字符，超限截断并标注。
- 越权边界：`epoch`（跨 epoch 回看）与 `receipt_id`（归档点读）组合一律
  显式报错（ToolCompleted exit_code 1 + error 字段），不静默回退；只读
  渲染无副作用。
- kill 动作面（形态核对结论）：主线模型面不暴露 `&` / `is_background`
  （T1.3 封闭 + `allow_background_operator=false`）；进程行带 `pid` +
  `killable`，中断动作经既有 run_terminal_cmd 对 PID 的命令语义
  （Unix `kill -9 <pid>` / Windows `taskkill /PID <pid> /F`）；可见后台
  逃生阀工具集若装配生命周期工具则同台可达（registry 既有机制）。
- 状态跃迁（start / auto-background / complete / idle 首现 / kill）沿用
  既有通知与事件链（BashExecutionBackgrounded / tool_running mid-run /
  TaskCompleted + idle_killed 形态）；live 分区本身不逐秒写事件。
- 验收点：渲染单测（行结构 / 摘要截断 / 8KiB 预算）+ 越权组合用例 +
  真实工具链回达 + host 事实映射 + 终端 live 快照用例。

## 2. 代码改动

### 2.1 orz-tools 终端 live 事实层

- `computer/types.rs`：新增 `TaskLiveSnapshot`（task_id / command /
  display_command / pid / elapsed_ms / status / total_bytes / cpu_micros /
  killable / owner_session_id / description）；`TerminalBackend` 增
  `list_live_tasks()`（默认空 = fail-closed，不支持 live 读取的后端不伪造
  状态）。
- `computer/local/terminal.rs`：
  - `ProcessState` 增 `pid` 字段（spawn 时固化，进程退出后仍可展示）；
  - 新 `TerminalCommand::ListLiveTasks` + actor `live_task_snapshots()`
    （读取时现算：CPU 进程树读数实时查、`idle` 状态来自 T1.5 采样器、
    elapsed 现算；status 取值 running / idle / completed / killed——
    waiting_input 无机械依据不产生，truthful fail-closed）；
  - `LocalTerminalBackend::list_live_tasks()` 实现。

### 2.2 orz-loop 黑板分区

- `host.rs`：新增 `LiveProcessFact` 与 `LoopHost::terminal_live_processes()`
  （默认空，fail-closed）。
- 新 `processes.rs`：纯渲染 `render_processes_text()`——`== processes
  (live) ==` 头、命令摘要 ≤80B（F4 cmd_preview 摘要纪律）、行字段齐备、
  整分区 ≤8KiB 字符、超限截断标注；空态「（无）」。
- `controller.rs`：`render_processes_section()`（live-only 守卫：epoch /
  receipt_id 显式 `Err` → exit_code 1 + error，O4 纪律）；`blackboard_read`
  工具定义 description 与 section 枚举加入 processes。
- `host_exec.rs`：`section=processes` 分支（异步向 host 取事实并渲染）；
  非法 section 错误文案、整响应 8KiB 强制边界同步。
- `blackboard.rs` 测试：serves 回达 + 越权组合两用例。

### 2.3 orz-host 事实接线

- `tools::build_toolset` 增 backend 参数（与 host 共享同一终端实例）；
- `OrzHost` 增 `terminal: Arc<dyn TerminalBackend>` 字段；
- `LoopHost::terminal_live_processes()` 映射终端快照 → `LiveProcessFact`。

## 3. 测试

| 位置 | 用例 | 覆盖 |
|---|---|---|
| processes.rs | `rows_carry_task_status_pid_and_killable` | 行结构/状态/killable |
| processes.rs | `command_preview_elides_to_80_chars` | 命令摘要 ≤80B + 省略号 |
| processes.rs | `oversized_section_is_truncated_with_marker` | 8KiB 预算截断 |
| processes.rs | `empty_section_renders_no_active_rows` | 空态（无） |
| terminal.rs | `test_list_live_tasks_reports_running_then_killed_states` | 实机：运行中行（pid/elapsed/killable/owner）→ idle-kill 后 killed |
| blackboard.rs | `blackboard_read_serves_processes_section` | 真实工具链回达 + section 枚举声明 |
| blackboard.rs | `blackboard_read_processes_combination_errors_are_explicit` | epoch/receipt_id 越权组合显式报错（事件面 exit 1 + error；模型面文本） |
| orz-host lib.rs | `terminal_live_processes_maps_terminal_snapshot` | host 事实映射（真实终端后台任务） |

## 4. 验证证据

- `cargo test -p orz-tools --lib computer::local::terminal::tests`：
  **48 passed / 0 failed**（5 ignored 为既有 flaky）。
- `cargo test -p orz-loop processes`：**6 passed / 0 failed**（渲染 4 +
  回达 1 + 越权 1）。
- `cargo test -p orz-host terminal_live_processes_maps_terminal_snapshot`：
  **1 passed**。
- `cargo check -p orz-loop -p orz-host`：净（仅既有告警）。
- `cargo fmt -p orz-tools -p orz-loop -p orz-host -- --check`：净。
- orz 仓库 `git diff --check`：exit 0。
- 说明：Linux 全量 + clippy + orz-assurance 归 M1 门 T1.13；Windows 沙箱
  grep/glob 全量失败属环境性（T1.3 已登记）。

## 5. 边界声明

- live-only：processes 不进黑板持久分区 / 会话存档（Blackboard 结构未
  增分区，`partition_revisions`/存档单包不受影响）；读取时现算即弃。
- 状态枚举：running / idle / completed / killed；waiting_input 无机械
  判定依据，truthful fail-closed 不产生（行文档已注明）。
- kill 动作面 = 进程行 pid + 既有 run_terminal_cmd PID 中断语义（模型面
  不暴露 `&` / `is_background`；可见逃生阀工具集如装配 kill_task 工具则
  同台可达）。
- `tool_running(status=idle_killed)` journal 侧接线（idle-kill 完成快照 →
  同 run 事件）仍随事件链集成归 T1.13 门（T0.2 schema/verifier/fixtures
  已就绪；本步终端已完成该快照形态，见 T1.5 审计边界）。
