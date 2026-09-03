# TER T1.3 模型面封闭实施审计（2026-09-04）

> 范围：TODO2 M1 步 T1.3——`hide_background_input` struct/serde 默认
> false→true（模型面封闭为 resident 默认）；显式 `&`/`is_background`
> 拒绝用例保持；验收 = 单测 + 工具 schema 无 `is_background` 暴露。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.1（参数默认值表）/ §10 S1-1（S5-2 常驻默认）。

## 1. 目标与验收

- struct `Default` 与 serde 缺省统一收敛：`hide_background_input=true`
  （单一来源，参照 T1.1 `auto_background_on_timeout` / T1.2
  `foreground_block_budget_ms` 同款处理）。
- orz-host 主线删除 `hide_background_input=true` 的冗余显式注入
  （缺省经 serde 解析即 true）。
- 显式 `&` / `is_background` 拒绝用例保持：`&` 由
  `allow_background_operator=false`（主线仍注入）拒绝；
  `is_background=true` 输入在默认封闭面被拒绝。
- 显式 `false` 逃生阀保留：需要「可见后台面」的工具集显式 opt-in，
  schema 显隐仍由有效 params 派生。
- 验收点：单测绿；run_terminal_cmd 默认导出 schema 无
  `is_background`（属性与 required 同步移除）。

## 2. 代码改动

### 2.1 orz-tools BashParams（单一生效源）

`crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs`：

- `hide_background_input`：`#[serde(default)]` → `#[serde(default =
  "default_true")]`；`impl Default` 内 false → true；字段 rustdoc 补
  TER T1.3 说明（封闭默认 + 显式 false 逃生阀 + requires_expr 联动）。

### 2.2 orz-host 主线参数

`crates/orz-host/src/tools.rs`：

- `run_terminal_cmd_tool_params()` 删除 `hide_background_input → true`
  注入项；函数 doc 更新为「T1.3 起由 BashParams resident 默认单一提供」。
- 保留注入：`allow_background_operator=false`（封 `&`）、
  `timeout_secs=600`、`max_timeout_secs=900`。
- `build_toolset` 内过时注释（“bash background mode … disabled”、
  与 T1.1 起 enabled_background 默认 true 事实矛盾）改写为 T1.3 口径：
  模型面封闭 + 内部 auto-bg（180s 中间回报 PID/落盘输出 + 完成提醒）
  承担可观察性/可取消性，不再要求被禁的后台任务工具。

## 3. 测试同步（默认翻转的既有依赖）

| 位置 | 用例/夹具 | 处理 |
|---|---|---|
| bash/mod.rs | `hide_background_input_rejects_explicit_background` | 删除显式 `true`，改为默认构造即拒绝（证明新默认） |
| bash/mod.rs | `background_command_starts`、`background_injects_python_unbuffered`、`tool_name_mapping_in_background_hint`、`background_hint_falls_back_when_get_output_tool_absent` | 新增 `make_resources_visible_background`（显式 false 逃生阀）并改挂 |
| bash/mod.rs | `make_resources_reject_bg_op` | 显式 `hide_background_input=false`（`&` 拒绝文案引导 `is_background=true`，属可见面语义） |
| bash/mod.rs | `schema_timeout_numbers_track_config`、`tool_description_timeout_numbers_track_config` | 显式 `hide_background_input=false`（这两例校验可见面历史文案/数字跟踪） |
| bash/mod.rs | `default_budget_180s_renders_in_model_facing_copy` | 改为纯默认构造；追加断言 schema 与工具描述均不含 `is_background` |
| registry/types.rs | `bash_definition_preserves_is_background_when_enabled` | params 补 `"hide_background_input": false`（可见面逃生阀） |
| registry/types.rs | `background_param_templates_reference_real_schema_keys` | run_terminal_cmd 补 `hide_background_input=false`（模板解析回归跑在可见面） |
| registry/types.rs | `bash_with_enabled_background_reports_missing_task_tools` | params None → `hide_background_input=false`（缺省已封闭，需显式重开才触发缺任务工具错误） |
| registry/types.rs | `bash_descriptions_track_system_reminders_setting` | run_terminal_cmd 补 `hide_background_input=false`（通知承诺/`is_background` 字段文案属可见面） |
| orz-workspace/hub.rs | `bg_config()` | run_terminal_cmd 改显式 `{enabled_background:true, hide_background_input:false}`（接线用例显式调 `is_background`） |

## 4. 新增测试

- bash/mod.rs `default_closes_is_background_surface`：默认构造
  hide=true；默认 exported schema 移除 `is_background`（属性 +
  required）；显式 false 重开属性与 required。
- bash/mod.rs `serde_omission_and_explicit_false_for_hide_background_input`：
  省略键 → true；显式 true / false 均可配。
- bash/mod.rs `default_budget_180s_renders_in_model_facing_copy` 强化：
  默认封闭面文案不出现 `is_background`，中间回报点仍渲染 after 180s。
- registry/types.rs `bash_definition_closed_by_default`：缺省 params
  的 run_terminal_cmd 可单独 finalize（不要求后台任务工具）、导出
  schema 无 `is_background`、描述不提及它（验收点落 registry 级）。
- orz-host tools.rs 联检断言：主线程 params 无 `hide_background_input`
  键；交给 BashParams serde 解析后 `hide_background_input==true`。

## 5. 附带修复：orz-workspace auto-bg 接线用例漂移（T1.2 遗留）

跑 orz-workspace 收窄回归时发现
`hub::tests::auto_background_on_timeout_increments_then_decrements_through_real_wiring`
在 T1.2 后失败：该用例原以「模型 timeout=300ms 触发 auto-bg」驱动，
而 T1.2 起有限预算语义下「解析超时 ≤ 预算 → kill-on-timeout」
（auto-bg 只在解析超时 > 预算时开启，避免“先回报后即杀”空序列）。
这是 T1.2 当时未跑 orz-workspace 留下的接线用例漂移。

处理：用例改为 500ms 短预算（`foreground_block_budget_ms=500`）+
模型 timeout=300s（> 预算），仍经真实 auto-bg 路径验证 activity
tracker 的增减与 idle 恢复；不改变 T1.2/T1.4 语义边界（T1.4 去硬杀
再处理 timeout 作为 auto-bg deadline 的断言改造）。

## 6. 验证证据

- `cargo test -p orz-tools --lib bash`：**229 passed / 0 failed**
  （覆盖 bash 模块、registry bash 表面、task_completion 相关全部用例；
  含新增 3 项 T1.3 用例）。
- `cargo test -p orz-host run_terminal_cmd_params_s5_2`：**1 passed**。
- orz-workspace hub 后台接线收窄：**7 passed / 0 failed**
  （backgrounded_bash / auto_background_on_timeout / monitor /
  update_tool_config / re_resolve_all_sessions / forked_child /
  concurrent_background_tasks）。
- `cargo fmt -p orz-tools -p orz-host -p orz-workspace -- --check`：净。
- `git -C D:\CLI\orz diff --check`：exit 0。

全量 `orz-tools --lib` 在本 Windows 沙箱另见 44 个 grep/glob 失败
（外部命令/拒绝访问，与 T1.3 无关，属环境性）；Linux 全量三件套 +
orz-loop/orz-assurance + clippy 归 M1 门 T1.13。

## 7. 边界声明

- 本步只翻转 resident 默认 + 删除冗余注入，不改拒绝文案、不强改
  行为路径；`allow_background_operator` 默认仍 true（field 文档既定
  兼容口径），主线由 `allow_background_operator=false` 显式封 `&`。
- 可见后台面（`is_background` 可见、需 task 工具同台）为显式 false
  逃生阀专属，测试夹具按此固定。
- schema 显隐由有效 params 派生：不另设静态 schema 默认。
- types.rs CRLF 与 orz-loop host_exec / orz-host local_browser 既有
  告警均为历史遗留，非本步引入。
