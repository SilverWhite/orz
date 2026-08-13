# 主 Agent 工作工具机械可用性探针 设计（2026-08-13，v0.2 修订）

> 状态：`approved`（2026-08-13 用户裁决放行实施；实施进度统一登记于
> [`BACKLOG_AND_PRIORITIES.md`](BACKLOG_AND_PRIORITIES.md) P0-A，步骤 1-7 与
> P0-A-2（v0.2 单一探针面扩展）均已闭合；ADR-0010 §3.5 修订已登记为 v1.8，
> 见 ADR §14.8；实施审计见 `docs/audits/GAP_TOOL_PROBE_V02_SINGLE_FACE_IMPL_AUDIT_2026-08-13.md`）。
> v0.2 修订（2026-08-13 用户裁决，定档）：**A 面与 C 面全部并入探针面（B）**——
> 单一规则：本轮模型可见 = 机械链路完整 ∩ 会话声明集；面 A/C 撤销，B 面升级为
> 全工作工具探针面。v0.1 的三面矩阵保留为历史语义，见 §2.1；当前设计以 §2.0 为准。
> 范围：设计定稿；实施按 P0-A 批次执行。
> 关联：ADR-0010 §3.5（2026-08-12 裁决"目录不承诺"；v0.2 修订正文登记随步骤 7）；
> `tool_availability_check` 事件（run-event v0.2，本设计对其做直接升级）；检索侧
> 机械控制（web_fetch 计数、机械预筛、引用校验）见
> [`RETRIEVAL_MECHANICAL_CONTROLS_DESIGN`](RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md)。
> 决策记录：2026-08-13 用户逐条裁决（两态中性判定、C 类定档、未配置后端调用时
> 声明、探针即用即清、ask_user_question 归 B 类、事件直接升级）；2026-08-13
> A+C→B 单一探针面定档。

## 1. 目标与不变量

- 目标：主 Agent 在规划时只面对"机械链路正常"的工作工具，减少因工具不可用导致的
  白烧轮与幻觉式重试；对探针不覆盖的工具保留显式调用审计。
- 不变量 1：模型可见面只有**工具名**，永不出现可用/不可用/成功/失败等判定词。
- 不变量 2：探针判定只对机械框架负责；调用时机械门禁仍是最终兜底。
- 不变量 3：探针结果是"探针时刻"的确定判定，不是调用成功承诺；真实调用结果回写修正。

## 2. 作用域与工具矩阵

### 2.0 v0.2 单一探针面（当前裁决，2026-08-13）

面 A/C 撤销；全部主 Agent 工作工具统一按“每动作机械探针 → 链路完整才进本轮
模型可见列表”治理：

| 工具 | 判定来源（机械链路） | 不完整 reason 示例（中性陈述） |
|---|---|---|
| `read_file` / `list_dir` / `grep` | 路径在 cwd 内存在且可读 | `工作区路径不可读` |
| `search_tool` | 与 grep 同源（索引/目录可读） | `工作区路径不可读` |
| `search_replace` | 目标路径可写 + 当前 permission profile 允许写 | `写权限策略未放行` |
| `run_tests` | `host.test_runner()` 存在 | `缺少测试运行器` |
| `ask_user_question` | 会话附着交互式用户（ACP live gateway） | `无交互式用户会话` |
| `blackboard_read` | journal/blackboard 存储可用 | `会话存储不可读` |
| `todo_write` / `update_goal` | 工作区可写 + goal/todo 上下文存在 | `工作区路径不可写` |
| `enter_plan_mode` / `exit_plan_mode` | 会话支持 plan 模式 | `会话不支持计划模式` |
| `compaction_whitelist_add` | 会话状态链可用 | `会话存储不可读` |
| `retrieval_disposition` | 检索子代理激活存在且有待处置（未决）assessment | `检索会话未激活` |
| `run_terminal_cmd` | 权限策略放行执行 + 宿主终端可用 | `终端链路不完整` |
| `lsp` | 工作区语言服务配置存在 | `语言服务未配置` |
| `memory_get` / `memory_search` | memory 显式 opt-in 且存储可用 | `记忆存储未启用` |
| `image_gen` / `image_edit` / `image_to_video` / `reference_to_video` | 后端已配置（provider/key） | `图像后端未配置` |
| `use_tool` | MCP/能力注册存在且会话作用域内 | `能力注册未配置` |

- 列表投影：`工具_defs = 探针完整集 ∩ registry 声明集`，仅工具名，无状态标注；
  registry 交集防止把会话不存在的工具声明给模型（2026-08-13 审查裁定）。
- 检索车道（`web_search` / `web_fetch` / `browser_read` / `pdf_read` /
  `project_doc_index`）与主车道非工作工具不参与本探针面，保持各自既有声明规则
  （子代理确定性留痕、模式门、宿主能力声明门）。
- 探针粒度为会话工作区根级机械检查（快照时刻无目标路径参数）；写探针为
  metadata-grade；调用时机械门禁仍是最终兜底（不变量 2）。

### 2.1 v0.1 三面矩阵（历史，已被 §2.0 取代，仅作实施过渡参考）

| 面 | 范围 | 探针 | 列表行为 |
|---|---|---|---|
| A. 恒可用 | `blackboard_read`、`todo_write`、`update_goal`、`enter_plan_mode`、`exit_plan_mode`、`compaction_whitelist_add`、`retrieval_disposition` | 无（无外部流程） | 固定列出，仅名称 |
| B. 探针过滤 | `read_file`、`list_dir`、`grep`、`search_tool`、`search_replace`、`run_tests`、`ask_user_question` | 每动作机械刷新 | 链路完整才列出，仅名称；不完整=确定不可提议 |
| C. 固定列表 | `run_terminal_cmd`、`lsp`、`memory_get`、`memory_search`、`image_gen`、`image_edit`、`image_to_video`、`reference_to_video`、`use_tool` | 无 | 固定列出，不做其他内容；调用时返回明确状态审计 |
| 检索车道（例外） | `web_search`、`web_fetch`、`browser_read`、`pdf_read`、`project_doc_index` | 无（子代理确定性留痕） | 保持现状：web_search 凭据构建期门、browser_read 能力探针声明门（含拒绝/宿主执行消息措辞，不适用本设计中性化规则） |

面 B 探针判定来源：

| 工具 | 判定来源 | 不完整 reason 示例（中性陈述） |
|---|---|---|
| read_file / list_dir / grep | 路径在 cwd 内存在且可读 | `工作区路径不可读` |
| search_replace | 目标路径可写 + 当前 permission profile 允许写 | `写权限策略未放行` |
| search_tool | 与 grep 同源（索引/目录可读） | `工作区路径不可读` |
| run_tests | `host.test_runner()` 存在 | `缺少测试运行器` |
| ask_user_question | 会话附着交互式用户 | `无交互式用户会话` |

面 C 未配置后端（image/video/memory/lsp）固定留在列表；调用时返回明确中性陈述
（如"未配置 image_gen 后端"），走调用状态审计，不做探针、不从列表移除。

实施边界（2026-08-13 审查裁定，步骤 1-2 落地语义）：

- 探针粒度为**会话工作区根级机械检查**（快照时刻无目标路径参数，探测
  session cwd 本身），不是逐路径检查；调用时机械门禁仍是最终兜底（不变量 2）。
- 写探针为 **metadata-grade**（目录可读 + readonly 属性），不做真实写测试；
  不构成写权限承诺。
- `ask_user_question` 的交互用户信号 = **ACP live gateway 存在**（headless
  与 hub-only 会话按不完整处理，fail-closed）。

## 3. 探针语义：两态中性判定

- 输出仅两态：**机械链路完整**（工具流程的全部机械组件齐备）→ 进列表；
  **机械链路不完整**（缺任一组件）→ 不进列表，reason 给中性陈述。
- 归并规则：原 `degraded`、`unprobed` 一律按"不完整"处理；探针实现缺失 =
  不完整 + "未完成链路检查"，fail-closed，不假设可用。
- 禁用词：`可用/不可用/成功/失败/available/unavailable/success/failure` 等
  判定词不进入**主车道**模型面兜底消息与探针 reason；机器 error 码/枚举、
  明确事实性错误与工具 schema 描述不受此限（2026-08-13 复核裁决）。
- 适用范围裁决（2026-08-13 复核）：中性化只用于防止"声明/列表与实际调用
  能力冲突"的主车道兜底面；检索车道保持原设计（消息措辞不脱敏，子代理正常
  留痕）；事件 error 码、机器 reason 与明确事实性错误可原样进模型面（有
  error 码即具体明确错误，不构成可用性冲突）；面 C 工具 schema 描述保留
  原样；denied/refused 等事实性门禁陈述不脱敏。
- reason 为稳定、机器可读的中性陈述（如"缺少测试运行器"），同时作为审计键。

## 4. 每动作刷新与列表投影

- 触发点：每个模型请求构造前（每个工具轮开始前），对面 B 全量重算快照。
- 成本：路径/权限/配置/宿主能力检查，微秒到低毫秒级；无网络、无进程拉起、无 TTL。
- 决策记录：早期讨论的"心跳探针"方案由每动作刷新取代——探针面每动作重算；
  检索车道不探。
- 即用即清：快照只在当轮请求构造时存在，用后即弃，不持久化、不跨轮保留。
- 最小上一轮映射：仅保留 `tool → 完整/不完整` 的上一轮最小映射，用于状态翻转对比；
  翻转才发 `tool_availability_check` 事件，无变化不发事件。
- 列表投影（v0.2）：`工具_defs = 探针完整集 ∩ registry 声明集`；仅工具名，
  无任何状态标注。registry 交集的必要性：部分会话变体可移除
  `ask_user_question`/`search_tool`（builder allowlist），探针不得把会话
  不存在的工具声明给模型（2026-08-13 审查裁定）。
- 同源约束（v0.2 并入 registry 交集语义）：探针不得把会话不存在的工具声明给
  模型；探针面之外的工具（检索车道、bash、宿主持有工具）不参与投影，保持各自
  既有声明规则。
- 确定不可提议：任一工作工具链路不完整 → 不进入本轮列表，模型无法提议；因竞态
  仍收到调用时，兜底返回中性机械陈述（如"tool 'run_tests' — 缺少测试运行器"）。

## 5. 调用时兜底与语义分离

- 模型面：只有工具名（原有描述/参数保留）。
- 兜底消息（主车道面 B/C）：`tool 'X' — <中性陈述>`，不使用判定词；检索
  车道拒绝/宿主执行消息保持原设计（见 §3 适用范围裁决）。
- 机械面：完整/不完整 + reason 进探针状态与事件。
- 审计面：真实调用失败照常走 ToolStarted/ToolCompleted(error) + 连续拒绝熔断；
  失败结果回写最小状态映射（调用即探针）。
- 回写车道边界（2026-08-13 审查裁定已落地）：调用即探针只属于拥有探针映射的
  主/grill 车道；检索车道不重算探针、不写主探针映射——车道内 Face B 读工具
  失败只走 ToolCompleted(error) 审计，不得污染主车道翻转事件流。
- v0.2 起无面 C：全部工作工具先探后列；调用时机械门禁与调用状态审计仍是最终
  兜底（对探针后竞态与后端瞬态均适用）。

## 6. run_tests 迁移

- 现状：controller 在 `host.test_runner().is_some()` 时追加声明（条件声明）。
- 迁移：删除条件声明逻辑，run_tests 进入面 B；探针 = runner 存在性（零成本、
  不启动测试进程）；链路完整才列出；兜底 reason `缺少测试运行器`。
- 对外行为不变，语义统一为探针过滤。
- 竞态兜底（2026-08-13 审查清理已落地）：无 runner 时若仍收到调用，在
  dispatch 前以无 ToolStarted 的显式拒绝返回中性陈述
  `tool 'run_tests' — 缺少测试运行器`（错误码 `missing_test_runner`），
  不再走默认 NotFound 执行路径。

## 7. tool_availability_check 事件直接升级

直接升级 v0.2 事件 payload（旧字段 `available/unavailable/degraded/unprobed`
移除，不保留兼容），新形状草案：

```json
{
  "probe_scope": "main_agent_work_tools",
  "probe_timestamp": "<RFC3339>",
  "complete": ["read_file", "list_dir", "grep", "search_tool", "search_replace", "run_tests"],
  "incomplete": [
    { "tool": "ask_user_question", "reason": "无交互式用户会话" }
  ],
  "gate_decision": "pass"
}
```

- `complete`/`incomplete` 覆盖全部工作工具（v0.2 单一探针面）；检索车道与非工作
  工具不出现。
- `gate_decision` 保留为机械框架内部字段（非模型面）。
- 迁移纪律：先 Schema/fixture/verifier 扩展，再改 producer，最后重捕真实 journal；
  v0.1 轨保持旧形状（replay-only）。

## 8. 探针状态与留痕

- 探针快照即用即清，不持久化、不落盘、不跨 run。
- 事件只在状态翻转时发；详情与 reason 不缓存。run-start 的首次事件视为
  "空映射 → 当前快照" 的首翻，必须先于 run_started 发出（Python 符合性），
  并作为本轮最小映射的播种值；之后无变化不发。
- 已有中间事件（tool_availability_check 翻转、ToolCompleted error、gate_decision）
  承担留痕，探针本身不留痕。

## 9. ADR-0010 §3.5 修订（v1.8，2026-08-13 已登记）

现裁决"模型可见工具列表=registry 全量、零可用性承诺、可用性完全调用时判定"
修订为（v0.2 单一探针面）：

> 主 Agent 工作工具统一由每动作机械探针治理：本轮模型可见 = 机械链路完整 ∩
> 会话声明集，仅工具名、不标注状态；链路不完整者确定不可提议。全部工作工具
> （含原恒声明控制工具与原固定列表工具）均按机械链路判定，不设免检面。调用时
> 机械判定仍为最终兜底。检索车道工具由子代理确定性留痕，不参与主探针矩阵。
> 探针输出只分"机械链路完整/不完整"，全线使用中性陈述。

该修订保留"调用时兜底"的根因防护，用"全量探针过滤 + 无状态标注"取代"全量列出
与三面分类"。已作为 ADR-0010 v1.8 §3.5 条 1/2 修订登记（2026-08-13，见 ADR §14.8）。

## 10. 实施边界与待办（登记，本轮不实施）

> 优先级：P0（当前工作集）。实施待办已统一迁至 [`BACKLOG_AND_PRIORITIES.md`](BACKLOG_AND_PRIORITIES.md)（P0/工具探针批次，含 7 项实施序列与决策门）；本文件不再单独维护待办明细，设计内容仍以本文为准。
>
> P0-A-2 闭合边界（2026-08-13）：单一探针面的全部 23 个判定已实现；host 可选
> 后端能力（lsp/memory/图像/视频/MCP）由 `LoopHost` fail-closed 默认值承载，
> orz-host 当前除终端外均未接线（`build_toolset` 固定禁用可选后端）——探针按
> 设计移除这些工具的可见性；未来接线任一后端时须同步翻转 orz-host 对应能力
> 访问器，并登记探针翻转测试。
>
> 2026-08-13 审查复核边界：`retrieval_disposition` 探针以"激活存在且携带未决
> pending assessment"为完整条件（Active 无 pending、continue 已决后均按不完整
> 处理；调用门禁仍为最终兜底）；plan 模式探针当前以交互用户信号代理，未来
> headless 但支持 plan 模式的会话需换独立能力信号。
