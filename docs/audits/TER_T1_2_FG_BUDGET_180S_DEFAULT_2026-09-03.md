# TER T1.2 首报/后台化预算默认 180s（2026-09-03）

> 上级：TODO2 T1.2（M1 orz 主线 / S1 工作包 1 第 2 步）；设计稿
> `docs/TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md` §10 S1 工作包
> 1（S5-2 常驻默认）+ §3.1（`foreground_block_budget_ms` 目标 180_000，
> 用户：3min）；单一生效源方案见
> [`TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md`](TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md)
> §3/§4。
> 状态：T1.2 完成（TODO2.md 已勾选）；范围仅限 `foreground_block_budget_ms`
> 生效默认收敛 180s（struct/serde 单源 + schema 描述同源 + 15s 后端默认
> 退役 + orz-host 300_000 注入删除）；T1.3（hide 默认 true）/ T1.4（timeout
> 分层去硬杀与静态 default/描述收敛）未动。

## 1. 改动（代码）

1. `orz-tools` grok_build `BashParams`：
   - `foreground_block_budget_ms` serde 缺省
     `#[serde(default)]` → `#[serde(default = "default_foreground_block_budget")]`
     （返回 `Some(DEFAULT_FOREGROUND_BLOCK_BUDGET_MS)`）；`impl Default`
     中 `None` → `Some(DEFAULT_FOREGROUND_BLOCK_BUDGET_MS)`；
   - 常量 `DEFAULT_FOREGROUND_BLOCK_BUDGET_MS` 15_000 → 180_000（取消
     `#[allow(dead_code)]`，serde 缺省/Default/null 回退/schema 描述同源）；
   - `effective_foreground_block_budget()` 的 `None` 分支不再「留给后端
     15s」，改为收敛 `Some(180s)`——auto_bg 开启时请求恒携带有限预算；
   - 字段/helper 文档同步（None 语义 = resident 180s，不再引用
     `GROK_FOREGROUND_BLOCK_BUDGET_MS` 缺省路径）。
2. schema/描述同源（消除“after 300s”硬编码）：
   - `exported_input_schema()` hide+auto-bg 分支的 timeout 描述由新 helper
     `auto_bg_property_mid_run_sentence()` 渲染——有限预算显示
     「after Ns」、预算 0（`Duration::MAX`）改为「resolved timeout 触发」；
   - `rendered_description()` 给模板注入 `auto_bg_mid_run_when` 从句
     （`after {secs}s and its timeout allows it` / `as its resolved timeout
     fires`），内部 auto-bg 工具描述同源渲染；
   - 内部模板「Long-running commands」bullet 用
     `${%- if auto_background_on_timeout %}` 守卫（关闭态不再误宣中间回报）；
   - 说明：两档默认超时「300s for ordinary / 600s for program」属 timeout
     分层文案，T1.4 收敛，本步未动。
3. `orz-tools` 终端后端 `computer/local/terminal.rs`：兜底常量
   `FOREGROUND_BLOCK_BUDGET` 15s → 180s（与 BashParams resident 默认同值；
   只兜底不带预算的直连 `TerminalRunRequest`）；`GROK_FOREGROUND_BLOCK_BUDGET_MS`
   env 覆盖保留为显式运维/测试旋钮，不再承担「15s 缺省」语义。
4. `orz-host` `run_terminal_cmd_tool_params()`：删除
   `foreground_block_budget_ms=300_000` 显式注入——缺省经 BashParams serde
   解析即 180_000（单一生效源，主线生效值 300s→180s 为预期行为变更）；
   测试改为断言该键不注入 + 将剩余主线 params 反序列化为 `BashParams`，
   budget 键缺省收敛 `Some(180_000)`。
5. 测试同步/新增：
   - `default_enables_auto_bg`：默认构造 budget=`Some(180_000)`、effective
     预算 `Some(180s)`；wait helper 断言改为
     `min(默认超时 120s, 180s)=120s`（并注明主线 timeout_secs 600/普通 300
     均 >180s，实际在 180s 后台化）；
   - `unset_budget_leaves_request_none_for_backend_default` →
     `explicit_null_budget_falls_back_to_180s_resident_default`（显式
     null/None 亦收敛 180s，不再走后端默认）；
   - `serde_accepts_foreground_block_budget_ms`：缺省解析
     `Some(180_000)`，显式 `null` 保持 None（effective 层回退 180s）；
   - 新增 `default_budget_180s_renders_in_model_facing_copy`（schema 与
     工具描述都渲染 after 180s、无 after 300s）；
   - MockTerminal 补前台请求捕获（`captured_run_request` +
     `success_capturing_run`），新增
     `default_budget_180s_materializes_auto_bg_foreground_request`：不传
     budget、解析超时 300s 时，`TerminalRunRequest` 携带
     `foreground_block_budget=Some(180s)` 且 `auto_background_on_timeout=true`；
   - terminal.rs 新增 `foreground_block_budget_backstop_is_180s`（后端 15s
     退役守卫）。

## 2. 验收对照

| 验收项 | 结果与证据 |
|---|---|
| 不传参数时 budget 生效默认 180_000 | `BashParams::default()` / serde 缺省均 `Some(180_000)`；`default_enables_auto_bg` + `serde_accepts_foreground_block_budget_ms` |
| struct 与 serde 单一来源 | serde `default_foreground_block_budget` 与 `Default` 共用 `DEFAULT_FOREGROUND_BLOCK_BUDGET_MS=180_000`；显式 `null` 在 effective 层同值回退 |
| 不传参数时前台命令 180s 触发 auto-bg | `default_budget_180s_materializes_auto_bg_foreground_request`：request 预算 `Some(180s)`、解析超时 300s>180s → auto-bg 开关 true；终端机制用例（custom 短预算）不回归 |
| schema 展示与生效值一致 | `default_budget_180s_renders_in_model_facing_copy`（after 180s）；budget=300_000 的既有 schema/工具描述用例渲染 after 300s（同源）；“after 300s”硬编码已删除 |
| 15s 后端默认消除 | `FOREGROUND_BLOCK_BUDGET` 15s→180s + `foreground_block_budget_backstop_is_180s` 守卫；mod.rs 常量/注释不再引用 15s 缺省 |
| orz-host 300_000 注入删除 | `run_terminal_cmd_params_s5_2_layered_timeout_and_mid_run`：键不注入 + 主线 params 解析回 `BashParams` budget=180_000 |
| 既有显式配置不回归 | `Some(0)`→`Duration::MAX`（timeout-only）、`Some(5000)`/`Some(600000)` min 语义、显式 false 关闭态等用例保持通过 |

## 3. 验证证据

- `cargo test -p orz-tools --lib bash` → **226 passed / 0 failed**（含
  registry、opencode、grok_build_concise 等 bash 相关，T1.1 224 + 新增 2）；
- `cargo test -p orz-tools --lib foreground_block_budget` → **18 passed /
  0 failed**（BashParams 预算/描述用例 + terminal 预算机制 3 例 + 常量
  守卫）；
- `cargo test -p orz-tools --lib default_budget_180s_materializes_auto_bg_foreground_request`
  → 1 passed（request 捕获验收点）；
- `cargo test -p orz-host run_terminal_cmd_params_s5_2` → 1 passed；
- `cargo fmt -p orz-tools -p orz-host -- --check` → 净（exit 0）；
- orz 仓库 `git diff --check`（本步 3 个改动文件：mod.rs / terminal.rs /
  tools.rs；types.rs 为 T1.1 遗留未提交改动）→ exit 0。
- 说明：clippy 与全量 orz-loop/orz-assurance 回归属 T1.13 M1 终验，不在
  本步重复跑。

## 4. 边界与后续

- 行为变更声明：orz 主线前台命令的中间回报点由 300s → **180s**（设计
  §3.1 目标值，用户裁定 3min）；这是 T1.2 的预期生效值变更，不是回归。
- 纯 struct 默认（无 host 注入）下 `timeout_secs=None→120s` 仍小于预算
  180s，此时 auto-bg 不会先于 timeout 触发（kill-on-timeout）；主线注入
  `timeout_secs=600` / 普通命令逐调用 300s 均 >180s，主线在 180s 触发
  auto-bg。struct 默认超时与 schema 静态 default（120s/300s/600s/900s）
  的收敛归 T1.4，不在本步。
- T1.3：`hide_background_input` 默认仍 false + orz-host 显式 true 注入
  保留；本次内部模板 guard 只防 auto-bg 关闭时误宣中间回报。
- 生成器 v0.2 表全面对齐（BACKLOG2 TER-0.1）不受影响。
