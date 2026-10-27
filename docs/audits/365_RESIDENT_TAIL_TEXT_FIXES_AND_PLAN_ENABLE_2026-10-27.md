# 365 批：0.9.14-2 验收后的尾部文案修复与 plan 工具启用

> 日期：2026-10-27
> 用户令：「现在 plan 的情况具体是什么样子的？……我不打算把 plan_first 机制拉回来了，就 plan 工具你想用的时候能用就行了」「请直接修掉 bug1 和 2 吧」。
> 基线：`086fef1d`（0.9.14-2）＋本批未提交改动。
> 性质：验收后补修；不改常驻策略与终态契约。

## §0 结论

0.9.14-2 的默认尾部清空与终态契约通过。本批修两处文案残留，并把 `plan`
从“休眠开关”接成“工具可用、无首轮门”：

1. 320K H1 去掉“模型间开始分化的位置”，改为“已越过 {k}K 估算线”。
2. 块表默认 pull 时，H1/T1/强制窗口不再说“见窗口尾部”，统一指向
   `.gsa/ledger/current.md`；soft/first-block 同步。
3. 生产 wiring 加 `.with_plan_enabled(true)`（CLI 一次性/TUI/ACP），
   **不**调用 `with_plan_gate_enabled`；`plan_write` 进入工具面，
   `<plan_framework>` 说明“复杂任务建议写，简单任务可直接行动；不强制首轮”。

## §1 验收证据（0.9.14-2）

- 版本：`orz 0.9.14-2`，`086fef1d`。
- 默认尾部：当前会话用户轮无 D4/块表/self_check；单轮实测回答“无，只有系统
  提示与工具定义”。
- env 回退：`ORZ_MODEL_FACE_D4=1` + `BLOCK_TABLE=1` + 低滑块，模型自报
  `[INITIAL_ROUND_INQUIRY]`、`[工作台]`、`【本 run 基线】`。
- 终态：`RUN-e3351166-4` `run_finished` completed/5 turns/68 tool rounds/
  `terminal_source=text_without_terminal`/`terminal_nudged=false`/
  `self_check_called=false`。
- 近两个 run 无 `tool_failed`/`model_error`/`error`。

## §2 本批改动

### 2.1 320K H1
`context_scale.rs::hard_reminder_block`：删“已越过模型间开始分化的位置”，
改为“已越过 {k}K 估算线（≈246K 真实 token）”。

### 2.2 块表指针
`model_face.rs` 新增 `BLOCK_TABLE_PULL_LINE` 与 `block_table_pointer_line()`；
`block_table_resident_enabled()` 关闭时返回 pull 指针，开启时仍返回
`BLOCK_TABLE_POINTER_LINE`。`context_scale.rs` 三处引用改走函数；
soft/first-block 的“窗口尾部/见下”改为“按需读取 `.gsa/ledger/current.md`”。

### 2.3 plan 工具
- `orz-bin/src/main.rs`（CLI 一次性 + TUI/交互）与
  `orz-host/src/acp_server.rs`（ACP 会话）加 `.with_plan_enabled(true)`。
- `with_plan_gate_enabled` 不调用：无首轮门。
- 旧 `with_plan_first_enabled` 仍为 alias（同时开 plan+gate），仅测试/回退用。

## §3 验证

- `cargo check -p orz-loop -p orz-bin -p orz-host`：通过（13.40s）。
- `cargo test -p orz-loop --lib`：**922 passed / 0 failed / 3 ignored**（10.65s）。
- `cargo build -p orz-bin --release`：`Finished release profile in 2m 36s`；
  `D:\CLI\orz\target\release\orz.exe`，`orz --version` = **0.9.14-3**。
- 冒烟（0.9.14-3，无 env）：默认尾部仍无 D4/块表/self_check；工具列表出现
  `plan_write`；模型看到 `<plan_framework>` 块（“计划可分步写、递交前核查、
  末步递交”，无“首轮必须”）。

## §4 边界

- 本批不恢复 plan_first 首轮门；`plan_gate` 仍 opt-in。
- 任务状态行仍 pull；`plan_write` 不依赖状态行。
- 分块表仍默认 pull；pull 面 `.gsa/ledger/current.md`（3.2MB），建议
  grep/块档案，不整文件读。
- RLI 提示不改：模型面是一行附注，趋势走 `blackboard_read section=rli`。

## §5 plan 现状（0.9.14-2 → 0.9.14-3）

| 版本 | `plan_write` | 首轮门 | 说明 |
|---|---|---|---|
| 0.9.14-2 | 无（11 工具） | 无（已退役） | plan 分区仍可读，但专用写入工具休眠 |
| 0.9.14-3 | 有（12+ 工具） | 无 | 工具随时可用；`<plan_framework>` 说明可选；状态行仍 pull |

`plan_first` 不再回来：`plan_gate` 仅在显式 `with_plan_gate_enabled(true)`
或旧 alias `with_plan_first_enabled(true)` 时启用；生产 wiring 只调用
`with_plan_enabled(true)`。旧 `PLAN_FIRST_FRAMEWORK_BLOCK` 仅作常量 alias。
