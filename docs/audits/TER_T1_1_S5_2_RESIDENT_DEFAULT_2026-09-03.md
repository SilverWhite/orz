# TER T1.1 S5-2 常驻默认开启（2026-09-03）

> 上级：TODO2 T1.1（M1 orz 主线 / S1 工作包 1 第 1 步）；设计稿
> `docs/TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md` §10 S1 工作包
> 1（S5-2 常驻默认）；单一生效源方案见
> [`TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md`](TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md)
> §3/§4。
> 状态：T1.1 完成（TODO2.md 已勾选）；范围仅限
> `auto_background_on_timeout` 常驻默认 + 其冗余注入清理；T1.2（budget
> 180s 单源）/ T1.3（hide 默认 true）未动。

## 1. 改动（代码）

1. `orz-tools` grok_build `BashParams`：
   - `auto_background_on_timeout` serde 缺省 `#[serde(default)]` →
     `#[serde(default = "default_true")]`（与 `enabled_background` 同款，
     缺省字段解析即 true）；
   - `impl Default` 中该字段 `false` → `true`（默认构造即 true）；
   - run 路径注释更新（旧「Backwards compatible because defaults to
     false」→ 新常驻默认语义：需要旧 kill-on-timeout 的调用方须显式
     `false`）。
2. `orz-host` `run_terminal_cmd_tool_params()`：删除 `enabled_background`
   与 `auto_background_on_timeout` 两个冗余显式 `true` 注入——按 T0.1
   核对表 §4，二者收敛为 BashParams struct/serde 默认单一来源；删除后
   缺省经 serde 解析仍为 true，**主线生效值不变**。`foreground_block_
   budget_ms=300_000`、`hide_background_input=true`、`allow_background_
   operator=false`、timeout 分层注入保留（T1.2/T1.3/T1.4 分别收敛）。
3. 测试同步：
   - `budget_none_when_auto_bg_off`（断言默认 false）拆为
     `default_enables_auto_bg`（默认 true + 缺省 budget 语义）与
     `explicit_false_disables_auto_bg`（显式 false 关闭态，budget/wait
     均 None）；
   - registry `types.rs` 两处仅设 `enabled_background=false` 的工具集
     测试补显式 `auto_background_on_timeout=false`（新默认下关闭态须
     显式，撞 params_constraint 校验）；
   - bash 模块两处关闭态构造同步补显式 false。

## 2. 验收对照

| 验收项 | 结果与证据 |
|---|---|
| 默认构造即 true | `default_enables_auto_bg`：`BashParams::default().auto_background_on_timeout == true`；serde 缺省经 `default_true` 同值 |
| 既有显式 false 用例不回归 | `explicit_false_disables_auto_bg`（budget/auto-bg wait 均 None）；registry 关闭态工具集、builder.rs 子代理禁用逃生阀（enabled+auto 双 false）、opencode/后端 TerminalRunRequest 显式 false 均原样保留 |
| 主线生效值不变 | orz-host 参数测试改为断言两键**不注入** + 其余 5 键逐字匹配；缺省 serde 解析 true |
| 关闭态仍可配 | 显式 false / enabled_background=false 组合按既有 params_constraint 校验执行 |

## 3. 验证证据

- `cargo test -p orz-tools --lib bash` → **224 passed / 0 failed**（含
  registry、grok_build_concise、opencode 相关）；
- `cargo test -p orz-tools --lib implementations::grok_build::bash` →
  **156 passed / 0 failed**；
- `cargo test -p orz-host run_terminal_cmd_params_s5_2` → 1 passed；
- `cargo fmt -p orz-tools -p orz-host -- --check` → 净（exit 0）；
- orz 仓库 `git diff --check`（3 个改动文件）→ exit 0。
- 说明：clippy 与全量 orz-loop/orz-assurance 回归属 T1.13 M1 终验，
  不在本步重复跑。

## 4. 边界与后续

- T1.2：`foreground_block_budget_ms` 默认收敛 180_000（struct/serde 单
  源 + 描述同源，消除 15s 后端默认与 300s 注入）；本步未动后端 15s 与
  300_000 注入，运行语义与 T1.1 前一致。
- T1.3：`hide_background_input` 默认 true（本次保持 false 默认与
  orz-host 显式 true 注入）。
- 生成器 v0.2 表全面对齐（BACKLOG2 TER-0.1）不受影响。
