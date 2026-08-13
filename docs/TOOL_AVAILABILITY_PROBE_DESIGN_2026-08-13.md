# 主 Agent 工作工具机械可用性探针 设计（2026-08-13，v0.1 定稿）

> 状态：`pending`（设计已冻结；实施与 ADR-0010 §3.5 修订待用户裁决）。
> 范围：本轮只做设计，不做具体实施。
> 关联：ADR-0010 §3.5（2026-08-12 裁决"目录不承诺"）；`tool_availability_check`
> 事件（run-event v0.2，本设计对其做直接升级）；检索侧机械控制（web_fetch 计数、
> 机械预筛、引用校验）见 [`RETRIEVAL_MECHANICAL_CONTROLS_DESIGN`](RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md)。
> 决策记录：2026-08-13 用户逐条裁决（两态中性判定、C 类定档、未配置后端调用时
> 声明、探针即用即清、ask_user_question 归 B 类、事件直接升级）。

## 1. 目标与不变量

- 目标：主 Agent 在规划时只面对"机械链路正常"的工作工具，减少因工具不可用导致的
  白烧轮与幻觉式重试；对探针不覆盖的工具保留显式调用审计。
- 不变量 1：模型可见面只有**工具名**，永不出现可用/不可用/成功/失败等判定词。
- 不变量 2：探针判定只对机械框架负责；调用时机械门禁仍是最终兜底。
- 不变量 3：探针结果是"探针时刻"的确定判定，不是调用成功承诺；真实调用结果回写修正。

## 2. 作用域与工具矩阵

| 面 | 范围 | 探针 | 列表行为 |
|---|---|---|---|
| A. 恒可用 | `blackboard_read`、`todo_write`、`update_goal`、`enter_plan_mode`、`exit_plan_mode`、`compaction_whitelist_add`、`retrieval_disposition` | 无（无外部流程） | 固定列出，仅名称 |
| B. 探针过滤 | `read_file`、`list_dir`、`grep`、`search_tool`、`search_replace`、`run_tests`、`ask_user_question` | 每动作机械刷新 | 链路完整才列出，仅名称；不完整=确定不可提议 |
| C. 固定列表 | `run_terminal_cmd`、`lsp`、`memory_get`、`memory_search`、`image_gen`、`image_edit`、`image_to_video`、`reference_to_video`、`use_tool` | 无 | 固定列出，不做其他内容；调用时返回明确状态审计 |
| 检索车道（例外） | `web_search`、`web_fetch`、`browser_read`、`pdf_read`、`project_doc_index` | 无（子代理确定性留痕） | 保持现状：web_search 凭据构建期门、browser_read 能力探针声明门 |

面 B 探针判定来源：

| 工具 | 判定来源 | 不完整 reason 示例（中性陈述） |
|---|---|---|
| read_file / list_dir / grep | 路径在 cwd 内存在且可读 | `工作区路径不可读` |
| search_replace / image_edit | 目标路径可写 + 当前 permission profile 允许写 | `写权限策略未放行` |
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
  判定词不进入模型面、兜底消息与事件 reason；机械框架内部枚举/日志可保留
  （如 GateDecision），对外序列化前映射为中性陈述。
- reason 为稳定、机器可读的中性陈述（如"缺少测试运行器"），同时作为审计键。

## 4. 每动作刷新与列表投影

- 触发点：每个模型请求构造前（每个工具轮开始前），对面 B 全量重算快照。
- 成本：路径/权限/配置/宿主能力检查，微秒到低毫秒级；无网络、无进程拉起、无 TTL。
- 决策记录：早期讨论的"心跳探针"方案由每动作刷新取代——面 B 每动作重算；
  面 C 与检索车道不探。
- 即用即清：快照只在当轮请求构造时存在，用后即弃，不持久化、不跨轮保留。
- 最小上一轮映射：仅保留 `tool → 完整/不完整` 的上一轮最小映射，用于状态翻转对比；
  翻转才发 `tool_availability_check` 事件，无变化不发事件。
- 列表投影：`工具_defs = 面A + (面B完整集 ∩ registry 声明集) + 面C`；仅工具名，
  无任何状态标注。registry 交集的必要性：部分会话变体可移除
  `ask_user_question`/`search_tool`（builder allowlist），探针不得把会话
  不存在的工具声明给模型（2026-08-13 审查裁定）。
- 确定不可提议：面 B 不完整 → 不进入本轮列表，模型无法提议；因竞态仍收到调用时，
  兜底返回中性机械陈述（如"tool 'run_tests' — 缺少测试运行器"）。

## 5. 调用时兜底与语义分离

- 模型面：只有工具名（原有描述/参数保留）。
- 兜底消息：`tool 'X' — <中性陈述>`，不使用判定词。
- 机械面：完整/不完整 + reason 进探针状态与事件。
- 审计面：真实调用失败照常走 ToolStarted/ToolCompleted(error) + 连续拒绝熔断；
  失败结果回写最小状态映射（调用即探针）。
- 面 C 工具：只有调用状态审计，不探不标。

## 6. run_tests 迁移

- 现状：controller 在 `host.test_runner().is_some()` 时追加声明（条件声明）。
- 迁移：删除条件声明逻辑，run_tests 进入面 B；探针 = runner 存在性（零成本、
  不启动测试进程）；链路完整才列出；兜底 reason `缺少测试运行器`。
- 对外行为不变，语义统一为探针过滤。

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

- `complete`/`incomplete` 只覆盖面 B（探针子集）；面 A/C 不出现。
- `gate_decision` 保留为机械框架内部字段（非模型面）。
- 迁移纪律：先 Schema/fixture/verifier 扩展，再改 producer，最后重捕真实 journal；
  v0.1 轨保持旧形状（replay-only）。

## 8. 探针状态与留痕

- 探针快照即用即清，不持久化、不落盘、不跨 run。
- 事件只在状态翻转时发；详情与 reason 不缓存。
- 已有中间事件（tool_availability_check 翻转、ToolCompleted error、gate_decision）
  承担留痕，探针本身不留痕。

## 9. ADR-0010 §3.5 修订建议（草案，待裁决）

现裁决"模型可见工具列表=registry 全量、零可用性承诺、可用性完全调用时判定"
修订为：

> 主 Agent 工作工具分三类治理：恒可用工具固定列出；本地确定性工具每动作机械探针
> 刷新，链路完整者进入可见列表（仅名称，不标注状态），不完整者确定不可提议；
> 本地进程/重进程/网络工具固定列出、不探不标，调用时返回明确状态并计入审计与
> 连续拒绝熔断。调用时机械判定仍为最终兜底。检索车道工具由子代理确定性留痕，
> 不参与主探针矩阵。探针输出只分"机械链路完整/不完整"，全线使用中性陈述。

该修订保留"调用时兜底"的根因防护，用"探针过滤 + 无状态标注"取代"全量列出"。

## 10. 实施边界与待办（登记，本轮不实施）

> 优先级：P0（当前工作集）。实施待办已统一迁至 [`BACKLOG_AND_PRIORITIES.md`](BACKLOG_AND_PRIORITIES.md)（P0/工具探针批次，含 7 项实施序列与决策门）；本文件不再单独维护待办明细，设计内容仍以本文为准。
