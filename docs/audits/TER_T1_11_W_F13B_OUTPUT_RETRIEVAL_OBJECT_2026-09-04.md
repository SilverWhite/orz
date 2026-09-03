# TER T1.11 W-F13b 输出检索对象实施审计（2026-09-04）

> 范围：TODO2 M1 步 T1.11——W-F13b：长输出落盘对象 + pattern / 行区间 /
> 尾部 N 行检索语义；ToolCompleted 截断标记 + 对象指针（schema 已 T0.2
> 定稿）。验收 = 检索语义单测；模型无需 .gsa 即可补读自身输出。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.6（W-F13）/ §10 S1-8；T0.2 schema/verifier（tool_completed v0.2
> 截断三字段配对规则）同目录审计；T1.10（64KB 档）为前置。

## 1. 目标与验收

- 每条（截断的）run_terminal_cmd 长输出以落盘 log 为**检索对象**：
  `output_object_id` = 落盘路径；提供 pattern / 行区间 / 尾部 N 行三类
  检索语义（模型侧直接用既有 read_file/grep 对对象操作即可，无需 .gsa
  摸黑）。
- ToolCompleted（v0.2 轨）在截断时带
  `output_truncated: true` + `total_bytes` + `output_object_id`
  （T0.2 配对：object_id ⇒ truncated + total_bytes）。
- 检索语义单测覆盖 pattern（大小写不敏感/上限）、行区间（1-based 闭区间
  + 越界 clamp）、尾部 N 行、缺失对象 IO 错误。

## 2. 代码改动

### 2.1 orz-tools 输出检索对象 API

`computer/output_object.rs`（新，pub）：

- `search_output_object(path, pattern, max_matches)` → `Vec<OutputObjectMatch>`
  （1-based 行号；大小写不敏感包含；0 = 不限量）；
- `line_range_output_object(path, start, end)`（1-based 闭区间；越界
  clamp）；
- `tail_output_object(path, n)`；
- 读取统一走固定解码链（GAP-ENCODING-GATE / OPS-PROTOCOL §8）。

### 2.2 orz-loop ToolResult / journal

- `host.rs`：`ToolResult` 增 `output_truncated` / `output_object:
  Option<TerminalOutputObject>`；新 `TerminalOutputObject { total_bytes,
  output_object_id }`（默认构造兼容 `..Default::default()`）。
- `host_exec.rs`：`tool_completed` payload 截断时落三字段（配对规则照
  T0.2）；console 重建路径透传同字段。

### 2.3 orz-host 映射

- `tools.rs`：`terminal_output_object_from_output()`——run_terminal_cmd
  前台完成且 `truncated` 时把 `total_bytes` + `output_file`（对象 id）
  映射为 `TerminalOutputObject`。
- `lib.rs`：`call_tool_inner` 设置 `output_truncated=true` +
  `output_object`（模型面截断指针文本原有 read_file 路径不变）。

## 3. 测试

| 位置 | 用例 | 覆盖 |
|---|---|---|
| output_object.rs | `pattern_search_is_case_insensitive_and_capped` | pattern 命中/行号/上限 |
| output_object.rs | `line_range_is_inclusive_and_clamped` | 行区间闭区间 + 越界 clamp |
| output_object.rs | `tail_returns_last_n_lines` | 尾部 N 行 |
| output_object.rs | `missing_object_is_an_io_error` | 缺失对象显式 IO 错误 |
| host_exec.rs | `tool_completed_carries_output_object_truncation_fields` | journal 三字段（output_truncated/total_bytes/output_object_id） |
| orz-host lib.rs | `run_terminal_cmd_truncation_carries_output_object` | 实机 30K 输出截断 → host 映射对象、落盘存在、模型面指针含 read_file |

## 4. 验证证据

- `cargo test -p orz-tools --lib output_object`：**4 passed**。
- `cargo test -p orz-loop tool_completed_carries_output_object_truncation_fields`：
  **1 passed**。
- `cargo test -p orz-host run_terminal_cmd_truncation_carries_output_object`：
  **1 passed**（真实命令 30K 输出）。
- **`cargo test -p orz-loop --lib`：704 passed / 0 failed / 3 ignored**
  （全量回归）。
- `cargo fmt -p orz-tools -p orz-loop -p orz-host -- --check`：净。
- orz 仓库 `git diff --check`：exit 0。

## 5. 边界声明

- 对象检索以落盘 log 为对象（output_file 路径即 object id）；既有
  read_file/grep 工具直接消费该对象，无需新增模型面工具（R1 保留面
  不变）；检索语义以公共 API 形态供工具/宿主侧复用（后续 W-F13 工具
  形态可挂载）。
- 后台任务（auto-bg/mid-run）未截断时无对象指针（`running:true` +
  task_id/output_file 语义已由 mid_run 承载）；截断三字段只出现在前台
  完成且截断的调用（配对规则与 T0.2 verifier 一致）。
- 完整输出文件保留在会话 `.gsa/session/terminal/`；对象指针即该路径，
  模型 read_file/grep 均可达（不再要求模型自行猜测/遍历 .gsa）。
