# TER T0.1 默认值收敛核对表（2026-09-03）

> 主题：工具执行层改革（TER）M0 设计门第 1 步——BashParams struct 默认 /
> 工具 schema 默认 / 后端 15s（GROK env）三处来源逐项核对，产出
> “现状→目标”表并标注单一生效源方案。本步为只读核对 + 清单产出，
> **未写任何业务代码**（放行签名在 T0.4，schema/verifier/fixtures 在
> T0.2）。
> 依据：[`TODO2.md`](../../TODO2.md) T0.1；设计稿
> [`TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md`](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.1/§3.7/§5/§10-S0；[`BACKLOG2.md`](../BACKLOG2.md) TER-0。

## 1. 核对结论（摘要）

1. **§3.1“现状默认”列混用两种口径**：`auto_background_on_timeout`
   （false）、`foreground_block_budget_ms`（None→后端 15s）、
   `hide_background_input`（false）三行是“struct/serde 默认，无 orz-host
   注入时”口径；`timeout_secs/max_timeout_secs`（300/600/900 分层）行是
   “orz-host 主线注入后有效”口径。三处来源必须逐参数分行核对，不能拿
   单一口径的默认值直接比较。
2. **orz 主线（orz-host `build_toolset`）已通过显式 JSON params 注入实现
   “常驻默认”**：`auto_background_on_timeout=true`、
   `foreground_block_budget_ms=300_000`、`hide_background_input=true`
   （S5-2，2026-08-29）。因此缺的不是主线有效行为，而是 **struct/serde
   默认本身仍是 false/None(→15s)/false**——orz“独立可用（无 harness/无
   host 注入）”时仍不自带常驻默认；这正是设计 P1 要收的口。
3. **schema 侧有三处漂移**：`BashToolInput.timeout` 静态 JSON Schema
   `default=120000`（与 orz 实际生效“普通 300s / 程序 600s”不符）；
   auto-bg 描述文案硬编码“after 300s”（与 `foreground_block_budget_ms`
   生效档位不同源）；`maximum=900000` 仅在 `max_timeout_secs` 显式配置
   时注入（当前主线配置了 900，schema 才显示 900）。
4. **15s 后端默认只在“请求未带 budget”时生效**：orz 主线显式带
   300_000，`GROK_FOREGROUND_BLOCK_BUDGET_MS`/`FOREGROUND_BLOCK_BUDGET`
   不参与；但 struct `None` 路径、bash 常量镜像
   `DEFAULT_FOREGROUND_BLOCK_BUDGET_MS=15_000` 仍指向 15s，是“独立默认”
   视角下的第二来源，需按 T1.2 消除。

## 2. 三处来源 + 主线注入定位

| 来源 | 代码位置 | 说明 |
|---|---|---|
| S1 BashParams struct/serde 默认 | [`grok_build/bash/mod.rs:140`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:140)（struct）、[`:219`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:219)（`Default`）、常量 [`:553`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:553)（`DEFAULT_MAX_TIMEOUT_MS`）、[`:561`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:561)（`DEFAULT_FOREGROUND_BLOCK_BUDGET_MS`）、[`:1056`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:1056)（`DEFAULT_TIMEOUT`） | 会话无任何配置时的缺省；`None` 语义 = 交给后端/内置默认 |
| S2 工具 schema 默认（模型面） | [`grok_build/bash/mod.rs:265`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:265)（`schema_default_timeout_ms`）、[`:282`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:282)（`timeout` 属性）、[`:1465`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:1465)（`exported_input_schema` 覆写 description/maximum/移除 `is_background`）、[`:1566`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:1566)（内部 auto-bg 描述模板） | 模型看到的 `timeout` 描述、default、maximum；`is_background` 显隐 |
| S3 后端 15s（GROK env） | [`computer/local/terminal.rs:55`](../../orz/crates/codegen/orz-tools/src/computer/local/terminal.rs:55)（`FOREGROUND_BLOCK_BUDGET=15s`）、[`:57`](../../orz/crates/codegen/orz-tools/src/computer/local/terminal.rs:57)（`GROK_FOREGROUND_BLOCK_BUDGET_MS` env）、[`:1104-1106`](../../orz/crates/codegen/orz-tools/src/computer/local/terminal.rs:1104)（request `None` → 后端默认）；请求字段 [`computer/types.rs:104`](../../orz/crates/codegen/orz-tools/src/computer/types.rs:104) | 仅当请求不带 `foreground_block_budget` 时生效（struct `None` 路径） |
| 主线注入层（orz-host，额外核对） | [`orz-host/src/tools.rs:115`](../../orz/crates/orz-host/src/tools.rs:115)（`run_terminal_cmd_tool_params`）、[`:158`](../../orz/crates/orz-host/src/tools.rs:158)（`terminal_tier_default_timeout_ms`：普通 300s/程序 600s）、[`:235`](../../orz/crates/orz-host/src/tools.rs:235)（`inject_terminal_default_timeout`）、[`:283`](../../orz/crates/orz-host/src/tools.rs:283)（`build_toolset` 应用点） | orz 主线的**有效**默认来源；逐调用按命令形态注入 timeout |
| 外层兜底（timeout 分层行用） | [`computer/local/terminal.rs:1835`](../../orz/crates/codegen/orz-tools/src/computer/local/terminal.rs:1835)（timeout 到点路径）、[`:1847`](../../orz/crates/codegen/orz-tools/src/computer/local/terminal.rs:1847)（默认杀进程）；`ORZ_TOOL_TIMEOUT_SECS` 读取点 [`orz-bin/src/main.rs:1229`](../../orz/crates/orz-bin/src/main.rs:1229)、[`orz-host/src/acp_server.rs:1999`](../../orz/crates/orz-host/src/acp_server.rs:1999) | 现行“超时即杀”硬杀路径与 orz 外层工具超时（评测面 900s） |

## 3. “现状→目标”核对表（覆盖 §3.1 参数表全行）

> 行内“现状”均为 2026-09-03 代码事实；“orz 主线现状”= `build_toolset`
> 注入后的有效值。

| 参数 | §3.1 目标默认 | S1 struct/serde 现状 | S2 schema/模型面现状 | S3 后端/GROK env 现状 | orz 主线现状 | 单一生效源方案 |
|---|---|---|---|---|---|---|
| `enabled_background` | true（不变） | true（`Default` + `#[serde(default="default_true")]`） | 非模型输入；主线 `hide_background_input=true` 时 `is_background` 属性被移除 | n/a（actor 内部开关） | true（显式注入） | **struct/serde 默认 true 为单一来源**；主线显式 `true` 注入随 T1.1 清理；子代理 task-deps 剔除分支的显式 `false` 逃生阀保留 |
| `auto_background_on_timeout` | true | false（`Default` + `#[serde(default)]`） | 非模型输入；auto-bg 行为文案由有效参数渲染 | n/a | true（显式注入） | **`Default` + serde 默认改 `default_true`**；主线显式注入删除后依赖 struct 默认；“要求 `enabled_background=true`”校验（mod.rs:240）与显式 `false` 用例（关闭态仍可配）保留 |
| `foreground_block_budget_ms` | 180_000 | None（常量镜像 15_000；生效走 S3） | 无独立 schema 属性；auto-bg 描述硬编码“after 300s”（[`mod.rs:1570-1571`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:1570)）与主线现值同源但**与目标档位不同步** | 15s + `GROK_FOREGROUND_BLOCK_BUDGET_MS`（请求 `None` 时生效） | Some(300_000)（FG 等待 300s） | **struct/serde 默认收敛为 180_000（schema+struct 单一来源，T1.2）**；消除 S3 独立 15s 后端默认（常量/ env 同步或退役）；描述“after 300s”改由生效值渲染 |
| `hide_background_input` | true | false（`Default` + `#[serde(default)]`） | false 时 `is_background` 属性可见；true 时 `exported_input_schema` 移除属性与 required | n/a | true（显式注入，属性隐藏） | **struct/serde 默认 true 为单一来源**；主线显式注入删除；显式 `&`/`is_background` 拒绝用例保持；schema 显隐由有效 params 派生（不另设静态默认） |
| `surface_bg_completion_reminders` | true（验证生效） | true（`Default` + `#[serde(default="default_true")]`） | 非模型输入；完成提醒 wording 为 system-reminder 顶部带回 | n/a | 未显式注入（继承 struct true） | **struct 默认 true 为单一来源**；生效验证闭环入 T1.x/T3.1（gcode 复跑，设计 §6.1） |
| `timeout_secs` / `max_timeout_secs`（timeout 分层） | 分层保留为 auto-bg deadline/上限引用；**不再作为杀进程点**（硬杀仅 idle-kill 与评测墙钟） | `timeout_secs=None`→`DEFAULT_TIMEOUT` 120s；`max_timeout_secs=None`→`DEFAULT_MAX_TIMEOUT_MS` 300s（5min 上限） | `timeout` JSON Schema 静态 `default=120000`；描述覆写“(max 900000)…default 300s ordinary/600s program…after 300s mid-run”（hide+auto-bg 形态）；`maximum=900000` 仅在显式 max 配置时注入 | terminal 到点即杀（[`terminal.rs:1847`](../../orz/crates/codegen/orz-tools/src/computer/local/terminal.rs:1847)）；orz 外层 `ORZ_TOOL_TIMEOUT_SECS`（评测 900s）兜底 | `timeout_secs=600`（程序缺省）/ `max_timeout_secs=900`（模型上限）；逐调用 `inject_terminal_default_timeout` 普通 300s·程序 600s | **分层数值收敛为 orz-host `terminal_tier_default_timeout_ms` 纯函数单一源**（schema 描述/静态 default/生效值三处同源，消除 120s 静态 default 漂移）；`max_timeout_secs` 收敛为 struct/注入单一口；T1.4 将 timeout 到期语义由“杀”改为“deadline 引用” |

## 4. 单一生效源方案汇总（供 T0.4 签名与 T1.1–T1.4 引用）

| 参数 | 收敛后唯一生效源 | 需退役/同步的其它源 | 实施步 |
|---|---|---|---|
| `enabled_background` | BashParams struct/serde 默认（true） | orz-host 显式 `true` 注入（冗余） | T1.1 |
| `auto_background_on_timeout` | BashParams struct/serde 默认（true） | orz-host 显式 `true` 注入；S2 无独立默认 | T1.1 |
| `foreground_block_budget_ms` | BashParams struct/serde 默认（180_000）+ schema 描述同源 | 后端 15s 常量/`GROK_FOREGROUND_BLOCK_BUDGET_MS` 独立默认；描述“after 300s”硬编码；orz-host 300_000 注入 | T1.2 |
| `hide_background_input` | BashParams struct/serde 默认（true） | orz-host 显式 `true` 注入 | T1.3 |
| `surface_bg_completion_reminders` | BashParams struct/serde 默认（true） | 无（需验证生效闭环） | T1.x/T3.1 |
| timeout 分层（普通/程序/上限） | orz-host `terminal_tier_default_timeout_ms` + `max_timeout_secs` 单一口 | schema 静态 `default=120000` 与描述漂移；timeout=杀进程路径 | T1.4 |

## 5. T1.x 联动单测/夹具锚点（改动时同步更新）

- [`grok_build/bash/mod.rs:4664`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:4664)：断言默认 `!auto_background_on_timeout`（默认值翻转后须改）；
- [`grok_build/bash/mod.rs:4670-4780`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:4670)：auto-bg/budget 显式值用例（300_000/600_000/0/2_000 等，T1.2 目标 180_000 后补默认构造用例）；
- [`grok_build/bash/mod.rs:4882`](../../orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs:4882)：serde 缺省/`Some(0)` 解析用例；
- [`computer/local/terminal.rs:3597`](../../orz/crates/codegen/orz-tools/src/computer/local/terminal.rs:3597)（`foreground_block_budget: None` 走后端默认）、[`:3716`](../../orz/crates/codegen/orz-tools/src/computer/local/terminal.rs:3716)（request 覆盖后端）：S3 默认退役后按 180s 单源重写；
- [`orz-host/src/tools.rs:750`](../../orz/crates/orz-host/src/tools.rs:750) `run_terminal_cmd_params_s5_2_layered_timeout_and_mid_run`：断言 300_000/600/900 注入值（T1.2/T1.4 改生效源后更新）；
- 设计 §5 风险 1 与 §10 S0 的“schema 默认（300s）vs struct（false/None/15s）”表述：按本表事实口径修正为“**schema 静态 default 120s + 描述 300/600s + struct None→15s + 主线注入 300s**”四处漂移（避免后续以错口径验收）。

## 6. 验收对照

- [x] 覆盖 §3.1 参数表全行（6 行）逐项核对；
- [x] 每行给出“现状→目标”与单一生效源方案；
- [x] 无业务代码改动（本步只产出核对表）。

> 状态登记：T0.1 完成（TODO2.md 勾选）；T0.2 schema/verifier/fixtures
> 先行、T0.3 ADR 候选、T0.4 放行签名待续。
