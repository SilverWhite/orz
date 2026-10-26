# 358 批：scout 只读子代理设计

> 日期：2026-10-26
> 用户令：「之前项目本身是有一个内部子代理设计的，只不过实现的时候终究还是没加，我考虑这一部分让你决断并进行吧」——决断权在助手，先文档后落码。
> 范围：只读 scout 子代理（不含 worker／多 agent 编排）。承接 354（系统控制台：capabilities／inspect／Envelope）。
> 计数：66（358 立项；357 之后首个新批）。

## §0 结论

现有 `RetrievalSubagent` 已经是一个**能跑完整 loop 的子代理**：`RoundAgent` 实现、独立 gateway、同父 run journal 的子代理事件、黑板独立分区、写域门禁（ADR-0010 §3.1 明文禁止"零工具无会话"的特例运行时）。缺的不是运行时，是**入口与契约**：主模型无法按需拉起一个通用 scout，也没有"结构化返回 + 证据"的返回面。

本批只做 **scout（只读）**：它用自己的窗口扫 N 个文件，回来只给一张 ≤2K 的 `findings[]` 表（每条带 `path:line` 证据与置信度）。**不做 worker（可写）**——写面并行会撕裂本仓库的单写者不变式（owner/lease、write control review、journal 哈希链、黑板分区顺序都假设一个写者）。

## §1 现状勘定

- `orz-loop/src/agents/retrieval.rs`：`SubagentRole { InternalRetrieval, ExternalRetrieval }`；`RetrievalSubagent::run_round` 与主 agent 同构；结果走 `[DOC]/[SOURCE]` 行契约，机械台账在 `retrieval/evidence.rs`。
- `orz-loop/src/agents/mod.rs`：黑板写属权已分（main→plan/exec，internal→internal_ret，external→external_ret，assurance→gate_log）。
- 缺口：
  1. 没有"主模型可调用"的启动入口（检索子代理由检索流程内部触发）；
  2. 没有通用任务契约（task/scope/budget）；
  3. 没有结构化返回面（现有是行契约，供检索域使用）；
  4. `capabilities`／`inspect` 未登记子代理 run。

## §2 决策

| 议题 | 决定 | 理由 |
|---|---|---|
| scout vs worker | **只做 scout** | 单写者不变式；worker 需要 worktree 隔离、审批归属、冲突合并三件新机制 |
| 触发方式 | 新增只读工具 `spawn_scout` | "方便模型使用"要求可发现、可调用；写黑板再轮询的间接触发不可靠 |
| 返回面 | 结构化 `findings[]` + `stats`，正文 ≤2K | 价值在**上下文压缩**：子代理烧自己的窗口，主模型只收表格 |
| 证据 | 每条 finding 必带 `path:line` + `confidence` | 防止"没证据的断言被当事实" |
| 深度 | 1（scout 不能再 spawn） | 防止递归放大与预算失控 |
| 预算 | `max_files / max_tokens / max_wall_s`，超限 fail-closed 返回 `partial` | 与 353 的"单工具超时"同一哲学：失败要可见 |

## §3 接口

### §3.1 工具签名

```
spawn_scout(
  task:   string,                 // 一句话目标，例："列出所有终端输出解码点"
  scope:  { paths: string[], tools?: string[] },  // 路径提示（advisory，不拦截）；工具缺省 read_file/grep/project_doc_index
  budget?: { max_files?: u32, max_tokens?: u32, max_wall_s?: u32 }
) -> ScoutReport
```

### §3.2 返回

```json
{ "report_id": "SC-0007",
  "status": "ok|partial|failed",
  "summary": "≤400 字结论",
  "findings": [ {"claim": "...", "evidence": "path:line", "confidence": "high|mid|low"} ],
  "stats": {"files_read": 12, "tokens_used": 18400, "wall_s": 42} }
```

正文 ≤2K 字符（summary ≤400 字）；全文落 `scout_report/<report_id>.md`，主模型按 id 用 `read_file` 取（不进窗口）。

### §3.3 错误

- 预算超限：`status=partial`，返回已得 findings，`stats` 如实（`max_wall_s` 到点时若有已得输出同样降级 partial；完全无输出才 failed）；
- 失败：`error: {class, message, last_step}`（class=timeout/agent_loop/parse）；
- 路径越界：按 354c 读面口径（读面已开放，敏感路径只提示不拦截）；
- 子 run 崩溃：`status=failed`，附 `error.class` 与最后一步。

## §4 运行时

- **同一 loop**：scout 走 `agent_loop`，`agent_role=scout`；子代理与主 run **同 journal**（不伪造独立目录）——wrapper `ToolStarted`/`ToolCompleted` 带 `child_run_id`/`parent_run_id`，子工具序列由 `inspect run` 的 `children[]` 按 wrapper 时间窗归组。
- **工具面白名单**：`read_file` / `grep` / `inspect`；**无写工具、无审批、无 browser、无 `web_search`**（外部检索已有专门车道）。
- **黑板**：scout 不写黑板分区；摘要携带 `index: SC-0007 | 12 files | 3 findings | <summary 60 字>` 行，供主模型按 §7 代写 `findings` 索引；全文在 `scout_report/`。
- **深度 1**：scout 的工具面里没有 `spawn_scout`。
- **并发**：同一 run 内至多 2 个 scout 并行（可配）；超出排队。

## §5 与现有组件的关系

- **retrieval subagent**：保留不动（领域专用、行契约、写 `internal_ret/external_ret`）。scout 是通用只读侦察，不替代它。
- **capabilities**：`spawn_scout` 登记为 `kind=read, readonly=true, schema_source=curated`（`params_schema=null`：声明在 loop 侧，host 无跨 crate 单源）；`inspect run` 在父 run 的 `children[]` 展示父子关系与子工具序列（不提供 `inspect run <child>` 独立入口）。
- **Envelope v1**：模型工具面不套 Envelope；`ToolResult.structured` 携 `ScoutReport`，journal 的 `ToolCompleted` 带 `chars/tokens/wall_s` 读数（CLI/控制台面按需包 Envelope）。

## §6 验收

1. `spawn_scout` 在 `capabilities` 出现且标 `readonly`；
2. 一次真实调用：12 个文件的盘点任务 → 返回 ≤2K、每条 finding 带 `path:line`；
3. `inspect run <parent> --json` 的 `children[]` 能看到子 run 的工具序列与终态；
4. scout 工具面里**没有** `search_replace`/`run_terminal_cmd`/`spawn_scout`（负向测试）；
5. 预算超限 → `status=partial` 且 `stats` 如实；
6. 主 run 的上下文增量 ≤2K（压缩效果可测）。

## §7 边界与不做

- 不做 worker（写面）、不做多 agent 编排、不做 scout 嵌套；
- 不让 scout 写黑板分区（只由主模型代写一行索引）；
- 不把 scout 结果直接注入主对话（只给表格与 id）。

## §8 分批

| 批 | 内容 |
|---|---|
| **358a** ✅ | 角色与运行时：`SubagentRole::Scout`、只读工具面白名单、子 run `child_run_id`（同 journal 逻辑子 run） |
| **358b** ✅ | 入口与契约：`spawn_scout` 工具、`ScoutReport`、`scout_report/` 落盘、capabilities 登记 |
| **358c** ✅ | 观测与收口：`inspect run` 父 `children[]`、预算与并发上限、负向测试与文档；审查修复 P0/P1/P2 |
